//! The Distribution page (versions, builds, review submissions, release) and the `Change` writes to App Store Connect.

use crate::{
  forms::{read_input, set_input},
  i18n::{tr, trf},
  theme::{colors, Colors, FONT_SIZE_SM},
  widgets::{card, state_badge},
  workspace::{Load, Workspace},
};
use appforge_appstore::{Client, Error};
use gpui_kit::{
  component::{
    alert::Alert,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputState},
    radio::RadioGroup,
    spinner::Spinner,
    tag::Tag,
    v_flex, Disableable as _, Sizable as _, WindowExt as _,
  },
  prelude::FluentBuilder as _,
  *,
};

/// How many builds and submissions show before "Show all".
const SHORT_LIST: usize = 4;

/// "2026-11-08T10:53:35.538Z" as "2026-11-08 10:53".
fn short_date(value: &str) -> String {
  value.replace('T', " ").chars().take(16).collect()
}

#[derive(Clone)]
pub(crate) enum Change {
  Create(String),
  Build(String, String),
  ReleaseType(String, bool),
  Schedule(String),
  EnablePhased(String),
  Phase(String, &'static str),
  Draft(String, String, Option<String>),
  Submit(String, usize),
  Cancel(String, usize),
  Release(String),
  PublishReviewResponse(String),
  DeleteReviewResponse(String),
}

impl Change {
  pub(crate) fn label(&self) -> &'static str {
    match self {
      Self::Create(_) => "createIosVersion",
      Self::Build(_, _) => "selectBuild",
      Self::ReleaseType(_, false) => "manualRelease",
      Self::ReleaseType(_, true) => "releaseAfterApproval",
      Self::Schedule(_) => "scheduleRelease",
      Self::EnablePhased(_) => "enablePhasedRelease",
      Self::Phase(_, "PAUSED") => "pausePhasedRelease",
      Self::Phase(_, _) => "resumePhasedRelease",
      Self::Draft(..) => "addToReviewDraft",
      Self::Submit(..) => "submitForReview",
      Self::Cancel(..) => "withdrawSubmission",
      Self::Release(_) => "releaseThisVersion",
      Self::PublishReviewResponse(_) => "publishReply",
      Self::DeleteReviewResponse(_) => "deleteReply",
    }
  }

  fn apply(&self, client: &Client, input: &str) -> Result<(), Error> {
    match self {
      Self::Create(app) => client.create_ios_version(app, input),
      Self::Build(version, build) => client.select_version_build(version, build),
      Self::ReleaseType(version, automatic) => client.set_release_type(version, None, *automatic),
      Self::Schedule(version) => client.set_release_type(version, Some(input), false),
      Self::EnablePhased(version) => client.enable_phased_release(version),
      Self::Phase(id, state) => client.set_phased_release_state(id, state),
      Self::Draft(app, version, draft) => client.add_version_for_review(app, version, draft.as_deref()),
      Self::Submit(id, _) => client.submit_for_review(id),
      Self::Cancel(id, _) => client.cancel_review(id),
      Self::Release(id) => client.release_version(id),
      Self::PublishReviewResponse(id) => client.save_customer_review_response(id, input),
      Self::DeleteReviewResponse(id) => client.delete_customer_review_response(id),
    }
  }
}

impl Workspace {
  pub(crate) fn load_distribution(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let (Some(client), Some(snapshot)) = (self.client.clone(), self.ready_snapshot()) else { return };
    let app = snapshot.app.id.clone();
    let version = snapshot.version.clone();
    let Some(data) = self.app_data.get_mut(&app) else { return };
    if data.distribution_task.is_some() {
      return;
    }
    if !matches!(data.distribution, Load::Ready(_)) {
      data.distribution = Load::Loading;
    }
    let target = app.clone();
    data.distribution_task = Some(cx.spawn_in(window, async move |this, cx| {
      let result = cx
        .background_spawn(async move {
          client.distribution(&target, version.as_ref().map(|version| (version.id.as_str(), version.version_string.as_str())))
        })
        .await;
      this
        .update_in(cx, |this, window, cx| {
          if this.require_apple_sign_in(&result, window, cx) {
            return;
          }
          let Some(data) = this.app_data.get_mut(&app) else { return };
          data.distribution_task = None;
          let active = this.selected.as_ref() == Some(&app);
          match result {
            Ok(distribution) => data.distribution = Load::Ready(distribution),
            Err(error) => {
              if !matches!(data.distribution, Load::Ready(_)) {
                data.distribution = Load::Failed(error.to_string().into());
              }
              if active {
                Self::error(window, cx, error.to_string());
              }
            }
          }
          if active {
            cx.notify();
          }
        })
        .ok();
    }));
    cx.notify();
  }

  pub(crate) fn apply_store_change(&mut self, change: Change, input: String, window: &mut Window, cx: &mut Context<Self>) {
    let (Some(client), Some(app)) = (self.client.clone(), self.selected.clone()) else { return };
    if self.store_write_busy || self.saving {
      return;
    }
    let customer_review = matches!(change, Change::PublishReviewResponse(_) | Change::DeleteReviewResponse(_));
    self.store_write_busy = true;
    self.tasks.store_write = Some(cx.spawn_in(window, async move |this, cx| {
      let worker = client.clone();
      let result = cx.background_spawn(async move { change.apply(&worker, input.trim()) }).await;
      this
        .update_in(cx, |this, window, cx| {
          this.store_write_busy = false;
          if this.client.as_ref().is_some_and(|current| std::sync::Arc::ptr_eq(current, &client)) {
            if this.require_apple_sign_in(&result, window, cx) {
              return;
            }
            match &result {
              Ok(()) => Self::success(window, cx, tr(this.settings.language, "savedToAppStore")),
              Err(error) => Self::error(window, cx, error.to_string()),
            }
            let active = this.selected.as_ref() == Some(&app);
            if let Some(data) = this.app_data.get_mut(&app) {
              if customer_review {
                data.reviews_task = None;
                if !active {
                  data.reviews = Load::Idle;
                }
              } else {
                data.distribution_task = None;
                data.distribution = Load::Idle;
                if result.is_ok() {
                  data.snapshot_task = None;
                  if !active {
                    data.snapshot = Load::Idle;
                  }
                }
              }
            }
            if active {
              if customer_review {
                this.load_customer_reviews(window, cx);
              } else if result.is_ok() {
                this.load_snapshot(window, cx);
              } else {
                this.ensure_app_page(window, cx);
              }
            }
          }
          cx.notify();
        })
        .ok();
    }));
    cx.notify();
  }

  pub(crate) fn distribution_button(&self, id: impl Into<ElementId>, change: Change, cx: &mut Context<Self>) -> Button {
    let l = self.settings.language;
    Button::new(id)
      .small()
      .outline()
      .label(tr(l, change.label()))
      .disabled(self.store_write_busy || self.saving)
      .on_click(cx.listener(move |this, _, window, cx| this.confirm_distribution_change(change.clone(), window, cx)))
  }

  fn confirm_distribution_change(&self, change: Change, window: &mut Window, cx: &mut Context<Self>) {
    let Some(snapshot) = self.ready_snapshot() else { return };
    let l = self.settings.language;
    let target: SharedString = match &snapshot.version {
      Some(version) if !matches!(change, Change::Create(_)) => format!("{} · iOS {}", snapshot.app.name, version.version_string).into(),
      _ => snapshot.app.name.clone().into(),
    };
    let count = match change {
      Change::Submit(_, count) | Change::Cancel(_, count) => Some(count.to_string()),
      _ => None,
    };
    let placeholder = match change {
      Change::Create(_) => Some("1.2.0"),
      Change::Schedule(_) => Some("2027-01-31T12:00:00Z"),
      _ => None,
    };
    let input = placeholder.map(|placeholder| cx.new(|cx| InputState::new(window, cx).placeholder(placeholder)));
    if let (Change::Schedule(id), Some(input)) = (&change, &input) {
      if let Some(Load::Ready(data)) = self.active_data().map(|data| &data.distribution) {
        if let Some(version) = data.versions.iter().find(|version| &version.id == id) {
          set_input(input, &version.release_date, window, cx);
        }
      }
    }
    let scheduled = matches!(change, Change::Schedule(_));
    let workspace = cx.entity();
    let label = change.label();
    window.open_dialog(cx, move |dialog, _, _| {
      let (workspace, change, field) = (workspace.clone(), change.clone(), input.clone());
      let footer = h_flex()
        .justify_end()
        .gap_2()
        .w_full()
        .child(Button::new("distribution-cancel").label(tr(l, "cancel")).outline().on_click(|_, window, cx| window.close_dialog(cx)))
        .child(Button::new("distribution-confirm").label(tr(l, change.label())).primary().on_click(move |_, window, cx| {
          let value = field.as_ref().map_or_else(String::new, |field| read_input(field, cx));
          window.close_dialog(cx);
          workspace.update(cx, |this, cx| this.apply_store_change(change.clone(), value, window, cx));
        }));
      dialog
        .title(tr(l, label))
        .child(
          v_flex()
            .gap_2()
            .child(target.clone())
            .children(count.as_ref().map(|count| trf(l, "itemsInThisSubmission", &[count])))
            .children(scheduled.then(|| tr(l, "releaseDateUtc")))
            .children(input.as_ref().map(Input::new)),
        )
        .footer(footer)
    });
  }

  /// One row of a list: content on the left, an action on the right.
  fn list_line(id: impl Into<ElementId>, highlighted: bool, c: &Colors) -> Stateful<Div> {
    h_flex().id(id).w_full().gap_3().px_3().py_2().items_center().rounded_md().when(highlighted, |row| row.bg(c.selected))
  }

  /// "Show all (12)" / "Show fewer", for a list that is cut short.
  fn expand_toggle(&self, key: &'static str, total: usize, cx: &mut Context<Self>) -> Option<Button> {
    if total <= SHORT_LIST {
      return None;
    }
    let l = self.settings.language;
    let open = self.distribution_expanded.contains(key);
    Some(
      Button::new(SharedString::from(format!("expand-{key}")))
        .label(if open { tr(l, "showFewer") } else { trf(l, "showAll", &[&total.to_string()]) })
        .ghost()
        .small()
        .on_click(cx.listener(move |this, _, _, cx| {
          if !this.distribution_expanded.remove(key) {
            this.distribution_expanded.insert(key);
          }
          cx.notify();
        })),
    )
  }

  pub(crate) fn render_distribution(&self, cx: &mut Context<Self>) -> AnyElement {
    let l = self.settings.language;
    let data = match self.active_data().map(|data| &data.distribution) {
      Some(Load::Ready(data)) => data,
      Some(Load::Failed(message)) => return Alert::error("distribution-error", message.clone()).into_any_element(),
      _ => return h_flex().gap_2().child(Spinner::new()).child(tr(l, "loadingFromAppStore")).into_any_element(),
    };
    let Some(snapshot) = self.ready_snapshot() else { return div().into_any_element() };
    let version = snapshot.version.as_ref();
    let c = colors(cx);
    let can_create = version.is_none_or(|version| {
      matches!(version.state.as_str(), "READY_FOR_SALE" | "READY_FOR_DISTRIBUTION" | "REMOVED_FROM_SALE" | "DEVELOPER_REMOVED_FROM_SALE")
    });

    // Versions ------------------------------------------------------------------------------------------------
    let mut versions = v_flex().gap_0p5();
    for item in &data.versions {
      let current = version.is_some_and(|version| version.id == item.id);
      versions = versions.child(
        Self::list_line(SharedString::from(format!("version-{}", item.id)), current, &c)
          .child(div().w_16().font_weight(FontWeight::SEMIBOLD).child(SharedString::from(item.number.clone())))
          .child(state_badge(&item.state, &c))
          .child(div().flex_1())
          .children(current.then(|| Tag::secondary().small().child(tr(l, "openVersion")))),
      );
    }
    let versions = card(
      tr(l, "iosVersions"),
      None,
      can_create.then(|| self.distribution_button("create-ios-version", Change::Create(snapshot.app.id.clone()), cx).primary().into_any_element()),
      versions,
      &c,
    );
    let Some(version) = version else {
      return v_flex().gap_4().child(versions).into_any_element();
    };
    let editable = version.is_editable();
    let id = &version.id;

    // Builds --------------------------------------------------------------------------------------------------
    let mut ordered: Vec<&appforge_appstore::StoreBuild> = data.builds.iter().collect();
    // The chosen build first, so it is never hidden by the short list.
    ordered.sort_by_key(|build| data.selected_build.as_ref() != Some(&build.id));
    let expanded = self.distribution_expanded.contains("builds");
    let shown = if expanded { ordered.len() } else { SHORT_LIST };
    let mut builds = v_flex().gap_0p5();
    if ordered.is_empty() {
      builds = builds.child(div().px_3().py_2().text_color(c.text_secondary).child(tr(l, "noMatchingBuilds")));
    }
    for build in ordered.iter().take(shown) {
      let selected = data.selected_build.as_ref() == Some(&build.id);
      let pickable = editable && !selected && !build.expired && !build.internal_only && build.state == "VALID";
      builds = builds.child(
        Self::list_line(SharedString::from(format!("build-{}", build.id)), selected, &c)
          .child(div().w_16().font_weight(FontWeight::SEMIBOLD).child(SharedString::from(build.number.clone())))
          .child(state_badge(&build.state, &c))
          .child(div().flex_1())
          .children(selected.then(|| Tag::custom(c.success.opacity(0.16), c.text, c.success).child(tr(l, "selectedBuild"))))
          .children(
            pickable
              .then(|| self.distribution_button(SharedString::from(format!("select-{}", build.id)), Change::Build(id.clone(), build.id.clone()), cx)),
          ),
      );
    }
    builds = builds.children(self.expand_toggle("builds", ordered.len(), cx));
    let builds = card(
      tr(l, "builds"),
      Some(if data.selected_build.is_some() { tr(l, "pickTheBuildThat") } else { tr(l, "noBuildIsSelected") }),
      None,
      builds,
      &c,
    );

    // App Review ----------------------------------------------------------------------------------------------
    let mut submissions: Vec<&appforge_appstore::ReviewSubmission> = data.submissions.iter().filter(|item| item.includes_version).collect();
    submissions.sort_by(|a, b| b.date.cmp(&a.date));
    let mut draft = None;
    for submission in &data.submissions {
      if submission.state == "READY_FOR_REVIEW" && !submission.includes_version && submission.item_count == 0 {
        draft = Some(submission.id.clone());
      }
    }
    let expanded = self.distribution_expanded.contains("reviews");
    let shown = if expanded { submissions.len() } else { SHORT_LIST };
    let mut review = v_flex().gap_0p5();
    for submission in submissions.iter().take(shown) {
      let operation = match submission.state.as_str() {
        "READY_FOR_REVIEW" => Some(Change::Submit(submission.id.clone(), submission.item_count)),
        "WAITING_FOR_REVIEW" | "IN_REVIEW" | "UNRESOLVED_ISSUES" => Some(Change::Cancel(submission.id.clone(), submission.item_count)),
        _ => None,
      };
      review = review.child(
        Self::list_line(SharedString::from(format!("submission-{}", submission.id)), false, &c)
          .child(state_badge(&submission.state, &c))
          .child(div().flex_1().text_size(FONT_SIZE_SM).text_color(c.text_secondary).child(short_date(&submission.date)))
          .children(operation.map(|change| self.distribution_button(SharedString::from(format!("review-{}", submission.id)), change, cx))),
      );
    }
    let active = data.submissions.iter().any(|item| item.includes_version && item.state != "COMPLETE");
    let can_draft = editable && !active;
    if submissions.is_empty() && !can_draft {
      review = review.child(div().px_3().py_2().text_color(c.text_secondary).child(tr(l, "nothingWasSubmittedFor")));
    }
    review = review.children(self.expand_toggle("reviews", submissions.len(), cx));
    let review = card(tr(l, "appReview"), None, None, review, &c);

    // Release -------------------------------------------------------------------------------------------------
    let current = data.versions.iter().find(|item| item.id == *id);
    let mode = current.map(|item| item.release_type.as_str()).unwrap_or_default();
    let mut release = v_flex().gap_3();
    let choices = [
      ("release-manual", Change::ReleaseType(id.clone(), false), mode == "MANUAL"),
      ("release-automatic", Change::ReleaseType(id.clone(), true), mode == "AFTER_APPROVAL"),
      ("release-schedule", Change::Schedule(id.clone()), mode == "SCHEDULED"),
    ];
    let checked = choices.iter().position(|(_, _, checked)| *checked);
    let labels = choices.iter().map(|(id, change, _)| gpui_kit::component::radio::Radio::new(*id).label(tr(l, change.label())));
    release = release.child(
      RadioGroup::vertical("release-options")
        .selected_index(checked)
        .disabled(!editable || self.store_write_busy || self.saving)
        .children(labels)
        .on_click(cx.listener(move |this, selected: &usize, window, cx| {
          if Some(*selected) != checked {
            this.confirm_distribution_change(choices[*selected].1.clone(), window, cx);
          }
        })),
    );
    if let Some(item) = current.filter(|item| !item.release_date.is_empty()) {
      release = release.child(div().text_size(FONT_SIZE_SM).text_color(c.text_secondary).child(trf(l, "scheduledFor", &[&item.release_date])));
    }
    if let Some((phase, state)) = &data.phased_release {
      let change = match state.as_str() {
        "ACTIVE" => Some(Change::Phase(phase.clone(), "PAUSED")),
        "PAUSED" => Some(Change::Phase(phase.clone(), "ACTIVE")),
        _ => None,
      };
      release = release.child(
        h_flex()
          .gap_3()
          .items_center()
          .child(div().font_weight(FontWeight::MEDIUM).child(tr(l, "phasedRelease")))
          .child(state_badge(state, &c))
          .children(change.map(|change| self.distribution_button("change-phased", change, cx))),
      );
    } else if editable && data.versions.iter().any(|item| matches!(item.state.as_str(), "READY_FOR_DISTRIBUTION" | "READY_FOR_SALE")) {
      release = release.child(h_flex().child(self.distribution_button("enable-phased", Change::EnablePhased(id.clone()), cx)));
    }
    let release_now = (version.state == "PENDING_DEVELOPER_RELEASE")
      .then(|| self.distribution_button("release-now", Change::Release(id.clone()), cx).primary().into_any_element());
    let release = card(tr(l, "release"), Some(tr(l, "howThisVersionGoes")), release_now, release, &c);

    // The next step, in one line at the top ------------------------------------------------------------------
    let (step, step_color): (SharedString, Hsla) = if version.state == "PENDING_DEVELOPER_RELEASE" {
      (tr(l, "approvedReleaseItWhen"), c.success)
    } else if matches!(version.state.as_str(), "WAITING_FOR_REVIEW" | "IN_REVIEW") {
      (tr(l, "inReviewAtApple"), c.info)
    } else if editable && data.selected_build.is_none() {
      (tr(l, "nextSelectABuild"), c.warning)
    } else if editable && !active {
      (tr(l, "nextAddThisVersion"), c.warning)
    } else if editable {
      (tr(l, "nextSubmitTheDraft"), c.warning)
    } else {
      (tr(l, "thisVersionIsLive"), c.success)
    };
    let banner = h_flex()
      .w_full()
      .gap_3()
      .px_4()
      .py_3()
      .items_center()
      .rounded_lg()
      .bg(step_color.opacity(0.12))
      .child(div().size_2().rounded_full().bg(step_color))
      .child(div().flex_1().font_weight(FontWeight::MEDIUM).child(step))
      .child(div().text_size(FONT_SIZE_SM).text_color(c.text_secondary).child(trf(l, "version", &[&version.version_string])))
      .child(state_badge(&version.state, &c))
      .children(can_draft.then(|| {
        let mut button = self.distribution_button("review-draft", Change::Draft(snapshot.app.id.clone(), id.clone(), draft), cx).primary();
        if data.selected_build.is_none() {
          button = button.disabled(true).tooltip(tr(l, "nextSelectABuild"));
        }
        button
      }));
    v_flex().gap_4().child(banner).child(builds).child(review).child(release).child(versions).into_any_element()
  }
}
