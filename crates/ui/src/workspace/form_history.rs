//! Form history scopes, saved revisions and restoring values into the current draft.

use super::*;
use crate::forms::set_area;
use appforge_storage::{History, HistoryRevision, HistoryValues};
use gpui_kit::component::{button::Button, h_flex, scroll::ScrollableElement as _, v_flex, Disableable as _, Icon, IconName, Sizable as _};

const HISTORY_DIALOG_WIDTH: Pixels = px(700.0);

const INFORMATION: &[&str] = &["appName", "subtitle"];
const LISTING: &[&str] = &["description", "keywords", "promotionalText", "supportUrl", "marketingUrl", "whatsNewInThis"];
const PRIVACY_LINKS: &[&str] = &["privacyPolicyUrl", "privacyChoicesUrl"];
const PRIVACY_ANSWERS: &[&str] = &[
  "iAnsweredAppPrivacy",
  "theAppCollectsData",
  "theAppTracksUsers",
  "theAppShowsThe",
  "peopleCanCreateAn",
  "accountsCanBeDeleted",
  "aReviewerMustSign",
  "dataTypesCollected",
  "thirdPartySdks",
];

#[derive(Default, PartialEq)]
pub(super) struct HistoryDraft {
  before: HistoryValues,
  after: HistoryValues,
  password: Option<(String, String)>,
}

impl HistoryDraft {
  pub(super) fn record(&mut self, label: &'static str, before: &str, after: &str, masked: bool) {
    if masked {
      self.password = Some((before.to_owned(), after.to_owned()));
    } else {
      self.before.insert(label.to_owned(), before.to_owned());
      self.after.insert(label.to_owned(), after.to_owned());
    }
  }

  pub(super) fn save(
    mut self,
    history: &History,
    secrets: &dyn SecretStore,
    tab: DetailTab,
    [info, listing, review, privacy]: [bool; 4],
  ) -> Result<(), String> {
    if !review {
      self.password = None;
    }
    let applied = |label: &str| match tab {
      DetailTab::Information => (info && INFORMATION.contains(&label)) || (listing && LISTING.contains(&label)),
      DetailTab::Privacy => (info && PRIVACY_LINKS.contains(&label)) || (privacy && PRIVACY_ANSWERS.contains(&label)),
      DetailTab::Review => review,
      _ => false,
    };
    self.after.retain(|label, _| applied(label));
    for (label, value) in &self.before {
      self.after.entry(label.clone()).or_insert_with(|| value.clone());
    }
    let passwords = self.password.as_ref().map(|(before, after)| (before.as_str(), after.as_str()));
    let password_changed = passwords.is_some_and(|(before, after)| before != after);
    if self.before == self.after && !password_changed {
      return Ok(());
    }
    let revision = HistoryRevision {
      before: self.before,
      after: self.after,
      password_changed,
      password_before_present: passwords.is_some_and(|(before, _)| !before.is_empty()),
      password_after_present: passwords.is_some_and(|(_, after)| !after.is_empty()),
    };
    history.append(&revision, passwords, secrets).map_err(|error| error.to_string())
  }
}

impl Workspace {
  fn history_scope(&self) -> Option<String> {
    let app = self.selected.as_deref()?;
    let issuer = self.settings.apple.as_ref()?.issuer_id.as_str();
    let (page, locale) = match self.tab {
      DetailTab::Information => ("information", self.locale.as_str()),
      DetailTab::Privacy => ("privacy", self.locale.as_str()),
      DetailTab::Review => ("review", ""),
      _ => return None,
    };
    Some(format!("{issuer}/{app}/{page}/{locale}"))
  }

  pub(super) fn form_history(&self) -> Option<History> {
    Some(History::new(&self.data_dir, &self.history_scope()?))
  }

  pub(super) fn record_local_history(&mut self, draft: HistoryDraft, window: &mut Window, cx: &mut Context<Self>) {
    let Some(history) = self.form_history() else { return };
    let secrets = self.secrets.clone();
    self.saving = true;
    self.tasks.save = Some(cx.spawn_in(window, async move |this, cx| {
      let result = cx.background_spawn(async move { draft.save(&history, secrets.as_ref(), DetailTab::Privacy, [false, false, false, true]) }).await;
      this
        .update_in(cx, |this, window, cx| {
          this.saving = false;
          this.tasks.save = None;
          if let Err(error) = result {
            Self::warning(window, cx, trf(this.settings.language, "historyRecordFailed", &[&error]));
          }
          cx.notify();
        })
        .ok();
    }));
    cx.notify();
  }

  pub(crate) fn form_history_button(&self, cx: &mut Context<Self>) -> Option<Button> {
    self.history_scope()?;
    let l = self.settings.language;
    Some(
      Button::new("form-history")
        .label(tr(l, "formHistory"))
        .icon(Icon::new(IconName::FileText))
        .small()
        .outline()
        .loading(self.tasks.form_history.is_some())
        .disabled(self.saving || self.tasks.form_history.is_some() || self.ready_snapshot().is_none())
        .on_click(cx.listener(|this, _, window, cx| this.open_form_history(window, cx))),
    )
  }

  fn open_form_history(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let (Some(scope), Some(history)) = (self.history_scope(), self.form_history()) else { return };
    self.tasks.form_history = Some(cx.spawn_in(window, async move |this, cx| {
      let result = cx.background_spawn(async move { history.list().map_err(|error| error.to_string()) }).await;
      this
        .update_in(cx, |this, window, cx| {
          this.tasks.form_history = None;
          cx.notify();
          if this.history_scope().as_ref() != Some(&scope) {
            return;
          }
          match result {
            Ok(entries) => this.show_history(scope, entries, window, cx),
            Err(error) => Self::error(window, cx, trf(this.settings.language, "historyReadFailed", &[&error])),
          }
        })
        .ok();
    }));
    cx.notify();
  }

  fn show_history(&self, scope: String, entries: Vec<(String, i64, HistoryRevision)>, window: &mut Window, cx: &mut Context<Self>) {
    let workspace = cx.weak_entity();
    let l = self.settings.language;
    window.open_dialog(cx, move |dialog, _, cx| {
      let c = crate::theme::colors(cx);
      let mut list = v_flex().id("form-history-list").w_full().max_h_96().overflow_y_scrollbar().gap_3();
      for (index, (id, time, revision)) in entries.iter().enumerate() {
        let restore = |before: bool| {
          let workspace = workspace.clone();
          let (id, scope, revision) = (id.clone(), scope.clone(), revision.clone());
          Button::new(SharedString::from(format!("restore-{id}-{before}")))
            .label(tr(l, if before { "restoreBeforeSave" } else { "restoreHistoryDraft" }))
            .small()
            .outline()
            .on_click(move |_, window, cx| {
              window.close_dialog(cx);
              workspace.update(cx, |this, cx| this.restore_history(scope.clone(), id.clone(), revision.clone(), before, window, cx)).ok();
            })
        };
        let date = chrono::DateTime::from_timestamp(*time, 0)
          .map_or_else(|| time.to_string(), |date| date.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M:%S").to_string());
        let changes = revision.after.iter().filter_map(|(label, after)| {
          let before = revision.before.get(label)?;
          (before != after).then(|| history_change(l, label, before, after, cx))
        });
        list = list.child(
          v_flex()
            .gap_2()
            .p_3()
            .rounded_lg()
            .border_1()
            .border_color(c.border)
            .child(
              h_flex()
                .gap_2()
                .flex_wrap()
                .justify_between()
                .items_center()
                .child(div().font_weight(FontWeight::SEMIBOLD).child(date))
                .child(h_flex().gap_2().flex_wrap().child(restore(false)).children((index + 1 == entries.len()).then(|| restore(true)))),
            )
            .children(changes)
            .children(revision.password_changed.then(|| div().text_sm().child(tr(l, "historyPasswordChanged")))),
        );
      }
      dialog.width(HISTORY_DIALOG_WIDTH).title(tr(l, "formHistory")).child(if entries.is_empty() {
        div().child(tr(l, "formHistoryEmpty")).into_any_element()
      } else {
        v_flex().gap_3().child(div().text_sm().text_color(c.text_secondary).child(tr(l, "formHistoryRestoreHelp"))).child(list).into_any_element()
      })
    });
  }

  fn restore_history(&mut self, scope: String, id: String, revision: HistoryRevision, before: bool, window: &mut Window, cx: &mut Context<Self>) {
    if self.history_scope().as_ref() != Some(&scope) || self.saving || self.tasks.form_history.is_some() {
      return;
    }
    let needs_password = if before { revision.password_before_present } else { revision.password_after_present };
    let values = if before { revision.before } else { revision.after };
    if !needs_password {
      self.fill_history_draft(&values, "", window, cx);
      return;
    }
    let secrets = self.secrets.clone();
    self.tasks.form_history = Some(cx.spawn_in(window, async move |this, cx| {
      let result = cx.background_spawn(async move { History::password(&id, before, secrets.as_ref()).map_err(|error| error.to_string()) }).await;
      this
        .update_in(cx, |this, window, cx| {
          this.tasks.form_history = None;
          cx.notify();
          if this.history_scope().as_ref() != Some(&scope) || this.saving {
            return;
          }
          match result {
            Ok(password) => this.fill_history_draft(&values, &password, window, cx),
            Err(error) => Self::error(window, cx, trf(this.settings.language, "historyReadFailed", &[&error])),
          }
        })
        .ok();
    }));
    cx.notify();
  }

  fn fill_history_draft(&mut self, values: &HistoryValues, password: &str, window: &mut Window, cx: &mut Context<Self>) {
    let forms = &mut self.forms;
    for (key, field) in [
      ("appName", &forms.name),
      ("subtitle", &forms.subtitle),
      ("keywords", &forms.keywords),
      ("supportUrl", &forms.support_url),
      ("marketingUrl", &forms.marketing_url),
      ("privacyPolicyUrl", &forms.privacy_url),
      ("privacyChoicesUrl", &forms.privacy_choices),
      ("firstName", &forms.first_name),
      ("lastName", &forms.last_name),
      ("phone", &forms.phone),
      ("email", &forms.email),
      ("demoUserName", &forms.demo_name),
      ("dataTypesCollected", &forms.data_types),
      ("thirdPartySdks", &forms.sdks),
    ] {
      if let Some(value) = values.get(key) {
        set_input(field, value, window, cx);
      }
    }
    for (key, field) in
      [("description", &forms.description), ("promotionalText", &forms.promotional), ("whatsNewInThis", &forms.whats_new), ("notes", &forms.notes)]
    {
      if let Some(value) = values.get(key) {
        set_area(field, value, window, cx);
      }
    }
    for (key, field) in [
      ("signInRequiredTo", &mut forms.demo_required),
      ("iAnsweredAppPrivacy", &mut forms.privacy.confirmed_in_app_store_connect),
      ("theAppCollectsData", &mut forms.privacy.collects_data),
      ("theAppTracksUsers", &mut forms.privacy.uses_tracking),
      ("theAppShowsThe", &mut forms.privacy.shows_tracking_prompt),
      ("peopleCanCreateAn", &mut forms.privacy.has_accounts),
      ("accountsCanBeDeleted", &mut forms.privacy.account_deletion_in_app),
      ("aReviewerMustSign", &mut forms.privacy.requires_login_to_review),
    ] {
      if let Some(value) = values.get(key) {
        *field = value == "saveValueYes";
      }
    }
    if self.tab == DetailTab::Review {
      set_input(&forms.demo_password, password, window, cx);
    }
    Self::success(window, cx, tr(self.settings.language, "historyDraftLoaded"));
    cx.notify();
  }
}

fn history_change(language: Language, label: &str, before: &str, after: &str, cx: &App) -> Div {
  let c = crate::theme::colors(cx);
  let value = |text: &str| match text {
    "" => tr(language, "saveValueEmpty"),
    "saveValueYes" if is_switch(label) => tr(language, "saveValueYes"),
    "saveValueNo" if is_switch(label) => tr(language, "saveValueNo"),
    text => SharedString::from(text.to_owned()),
  };
  v_flex().gap_1().child(div().text_sm().font_weight(FontWeight::MEDIUM).child(crate::i18n::translated_key(language, label))).child(
    h_flex()
      .gap_2()
      .items_start()
      .child(div().flex_1().min_w_0().text_sm().text_color(c.text_secondary).child(value(before)))
      .child(Icon::new(IconName::ArrowRight).small().text_color(c.text_secondary))
      .child(div().flex_1().min_w_0().text_sm().child(value(after))),
  )
}

fn is_switch(label: &str) -> bool {
  label == "signInRequiredTo" || PRIVACY_ANSWERS[..7].contains(&label)
}
