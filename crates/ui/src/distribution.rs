use crate::{
  forms::read_input,
  i18n::{tr, trf},
  theme::colors,
  widgets::{section, state_badge},
  workspace::{DetailTab, Load, Workspace},
};
use appforge_appstore::{Client, Error};
use gpui_kit::{
  component::{
    alert::Alert,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputState},
    spinner::Spinner,
    v_flex, Disableable as _, Sizable as _, WindowExt as _,
  },
  *,
};

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
      Self::Create(_) => "Create iOS version",
      Self::Build(_, _) => "Select build",
      Self::ReleaseType(_, false) => "Manual release",
      Self::ReleaseType(_, true) => "Release after approval",
      Self::Schedule(_) => "Schedule release",
      Self::EnablePhased(_) => "Enable phased release",
      Self::Phase(_, "PAUSED") => "Pause phased release",
      Self::Phase(_, _) => "Resume phased release",
      Self::Draft(..) => "Add to review draft",
      Self::Submit(..) => "Submit for review",
      Self::Cancel(..) => "Withdraw submission",
      Self::Release(_) => "Release this version",
      Self::PublishReviewResponse(_) => "Publish reply",
      Self::DeleteReviewResponse(_) => "Delete reply",
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
    self.distribution = Load::Loading;
    self.tasks.distribution = Some(cx.spawn_in(window, async move |this, cx| {
      let result = cx
        .background_spawn(
          async move { client.distribution(&app, version.as_ref().map(|version| (version.id.as_str(), version.version_string.as_str()))) },
        )
        .await;
      this
        .update_in(cx, |this, window, cx| {
          this.distribution = match result {
            Ok(distribution) => Load::Ready(distribution),
            Err(error) => {
              Self::error(window, cx, error.to_string());
              Load::Failed(error.to_string().into())
            }
          };
          cx.notify();
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
            match &result {
              Ok(()) => Self::success(window, cx, tr(this.settings.language, "Saved to App Store Connect.")),
              Err(error) => Self::error(window, cx, error.to_string()),
            }
            if this.selected.as_ref() == Some(&app) {
              if customer_review {
                this.load_customer_reviews(window, cx);
              } else if result.is_ok() {
                this.load_snapshot(window, cx);
              } else if this.tab == DetailTab::Distribution {
                this.load_distribution(window, cx);
              }
            }
          }
          cx.notify();
        })
        .ok();
    }));
    cx.notify();
  }

  fn distribution_button(&self, id: impl Into<ElementId>, change: Change, cx: &mut Context<Self>) -> Button {
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
    let workspace = cx.entity();
    let label = change.label();
    window.open_dialog(cx, move |dialog, _, _| {
      let (workspace, change, field) = (workspace.clone(), change.clone(), input.clone());
      let footer = h_flex()
        .justify_end()
        .gap_2()
        .w_full()
        .child(Button::new("distribution-cancel").label(tr(l, "Cancel")).outline().on_click(|_, window, cx| window.close_dialog(cx)))
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
            .children(count.as_ref().map(|count| trf(l, "{} items in this submission", &[count])))
            .children(input.as_ref().map(Input::new)),
        )
        .footer(footer)
    });
  }

  pub(crate) fn render_distribution(&self, cx: &mut Context<Self>) -> AnyElement {
    let l = self.settings.language;
    let data = match &self.distribution {
      Load::Ready(data) => data,
      Load::Failed(message) => return Alert::error("distribution-error", message.clone()).into_any_element(),
      _ => return h_flex().gap_2().child(Spinner::new()).child(tr(l, "Loading from App Store Connect…")).into_any_element(),
    };
    let Some(snapshot) = self.ready_snapshot() else { return div().into_any_element() };
    let version = snapshot.version.as_ref();
    let palette = colors(cx);
    let mut history = v_flex().gap_2();
    if version.is_none_or(|version| {
      matches!(version.state.as_str(), "READY_FOR_SALE" | "READY_FOR_DISTRIBUTION" | "REMOVED_FROM_SALE" | "DEVELOPER_REMOVED_FROM_SALE")
    }) {
      history = history.child(self.distribution_button("create-ios-version", Change::Create(snapshot.app.id.clone()), cx));
    }
    for item in &data.versions {
      history = history.child(h_flex().gap_2().child(SharedString::from(item.number.clone())).child(state_badge(&item.state, &palette)));
    }
    let page = v_flex().gap_6().child(section(tr(l, "iOS versions"), history));
    let Some(version) = version else { return page.into_any_element() };
    let editable = version.is_editable();
    let id = &version.id;
    let mut builds = v_flex().gap_2();
    if data.builds.is_empty() {
      builds = builds.child(tr(l, "No matching builds."));
    }
    for build in &data.builds {
      let selected = data.selected_build.as_ref() == Some(&build.id);
      builds = builds.child(
        h_flex()
          .gap_2()
          .child(SharedString::from(build.number.clone()))
          .child(state_badge(&build.state, &palette))
          .children(selected.then(|| tr(l, "Selected build")))
          .children(
            (editable && !selected && !build.expired && !build.internal_only && build.state == "VALID")
              .then(|| self.distribution_button(SharedString::from(format!("select-{}", build.id)), Change::Build(id.clone(), build.id.clone()), cx)),
          ),
      );
    }
    let mut review = v_flex().gap_2();
    let mut draft = None;
    for submission in &data.submissions {
      if submission.state == "READY_FOR_REVIEW" && !submission.includes_version && submission.item_count == 0 {
        draft = Some(submission.id.clone());
      }
      if submission.includes_version {
        let operation = match submission.state.as_str() {
          "READY_FOR_REVIEW" => Some(Change::Submit(submission.id.clone(), submission.item_count)),
          "WAITING_FOR_REVIEW" | "IN_REVIEW" | "UNRESOLVED_ISSUES" => Some(Change::Cancel(submission.id.clone(), submission.item_count)),
          _ => None,
        };
        review = review.child(
          h_flex()
            .gap_2()
            .child(state_badge(&submission.state, &palette))
            .child(SharedString::from(submission.date.clone()))
            .children(operation.map(|change| self.distribution_button(SharedString::from(format!("review-{}", submission.id)), change, cx))),
        );
      }
    }
    if editable && data.selected_build.is_some() && !data.submissions.iter().any(|item| item.includes_version && item.state != "COMPLETE") {
      review = review.child(self.distribution_button("review-draft", Change::Draft(snapshot.app.id.clone(), id.clone(), draft), cx));
    }
    let mut release = v_flex().gap_2();
    if let Some(current) = data.versions.iter().find(|item| item.id == *id) {
      let mode = match current.release_type.as_str() {
        "MANUAL" => Some("Manual release"),
        "AFTER_APPROVAL" => Some("Release after approval"),
        "SCHEDULED" => Some("Scheduled release"),
        _ => None,
      };
      release = release.children(mode.map(|mode| tr(l, mode))).child(SharedString::from(current.release_date.clone()));
    }
    if editable {
      release = release.child(
        h_flex()
          .gap_2()
          .flex_wrap()
          .child(self.distribution_button("release-manual", Change::ReleaseType(id.clone(), false), cx))
          .child(self.distribution_button("release-automatic", Change::ReleaseType(id.clone(), true), cx))
          .child(self.distribution_button("release-schedule", Change::Schedule(id.clone()), cx)),
      );
    }
    if version.state == "PENDING_DEVELOPER_RELEASE" {
      release = release.child(self.distribution_button("release-now", Change::Release(id.clone()), cx));
    }
    if let Some((phase, state)) = &data.phased_release {
      let change = match state.as_str() {
        "ACTIVE" => Some(Change::Phase(phase.clone(), "PAUSED")),
        "PAUSED" => Some(Change::Phase(phase.clone(), "ACTIVE")),
        _ => None,
      };
      release = release.child(
        h_flex()
          .gap_2()
          .child(tr(l, "Phased release"))
          .child(state_badge(state, &palette))
          .children(change.map(|change| self.distribution_button("change-phased", change, cx))),
      );
    } else if editable && data.versions.iter().any(|item| matches!(item.state.as_str(), "READY_FOR_DISTRIBUTION" | "READY_FOR_SALE")) {
      release = release.child(self.distribution_button("enable-phased", Change::EnablePhased(id.clone()), cx));
    }
    page
      .child(section(tr(l, "Builds"), builds))
      .child(section(tr(l, "App Review"), review))
      .child(section(tr(l, "Release"), release))
      .into_any_element()
  }
}
