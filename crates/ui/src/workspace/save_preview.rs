//! Builds a save plan from the current draft and previews the exact fields it changes.

use super::*;
use crate::theme::colors;
use gpui_kit::component::{
  button::{Button, ButtonVariants as _},
  h_flex,
  scroll::ScrollableElement as _,
  v_flex, Icon, IconName, Sizable as _,
};

const SAVE_PREVIEW_WIDTH: Pixels = px(640.0);

#[derive(Default, PartialEq)]
struct SavePlan {
  changes: Vec<Change>,
  history: Option<super::form_history::HistoryDraft>,
  info: Option<(AppInfoLocalization, AppInfoLocalization)>,
  listing: Option<(VersionLocalization, VersionLocalization)>,
  review: Option<(String, ReviewDetail)>,
  privacy: Option<PrivacyDeclaration>,
}

#[derive(PartialEq)]
struct Change {
  label: &'static str,
  before: String,
  after: String,
  kind: ValueKind,
}

#[derive(Clone, Copy, PartialEq)]
enum ValueKind {
  Text,
  Masked,
  Boolean,
}

#[derive(Default)]
struct SavedFields {
  info: Option<AppInfoLocalization>,
  listing: Option<VersionLocalization>,
  review: Option<(String, ReviewDetail)>,
}

impl SavePlan {
  fn record(&mut self, label: &'static str, before: &str, after: &str, kind: ValueKind) {
    if let Some(history) = &mut self.history {
      history.record(label, before, after, kind == ValueKind::Masked);
    }
    if before != after {
      self.changes.push(Change { label, before: before.into(), after: after.into(), kind });
    }
  }

  fn text(&mut self, label: &'static str, original: &str, input: String) -> String {
    self.text_with_mask(label, original, input, false)
  }

  fn text_with_mask(&mut self, label: &'static str, original: &str, input: String, masked: bool) -> String {
    let edited = if original.trim() == input { original.to_owned() } else { input };
    self.record(label, original, &edited, if masked { ValueKind::Masked } else { ValueKind::Text });
    edited
  }

  fn boolean(&mut self, label: &'static str, before: bool, after: bool) {
    self.record(label, if before { "saveValueYes" } else { "saveValueNo" }, if after { "saveValueYes" } else { "saveValueNo" }, ValueKind::Boolean);
  }

  fn has_remote_changes(&self) -> bool {
    self.info.is_some() || self.listing.is_some() || self.review.is_some()
  }
}

impl Workspace {
  pub(crate) fn privacy_draft(&self, cx: &App) -> PrivacyDeclaration {
    PrivacyDeclaration {
      data_types: read_input(&self.forms.data_types, cx),
      third_party_sdks: read_input(&self.forms.sdks, cx),
      ..self.forms.privacy.clone()
    }
  }

  pub(crate) fn has_pending_save(&self, cx: &App) -> bool {
    !self.save_plan(cx, false).changes.is_empty()
  }

  fn save_plan(&self, cx: &App, capture_history: bool) -> SavePlan {
    let mut plan = SavePlan { history: capture_history.then(Default::default), ..Default::default() };
    let Some(snapshot) = self.ready_snapshot() else { return plan };
    let forms = &self.forms;
    let input = |field| read_input(field, cx);
    let area = |field| read_area(field, cx);

    if matches!(self.tab, DetailTab::Information | DetailTab::Privacy) {
      if let Some(original) = snapshot.info_localizations.iter().find(|item| item.locale == self.locale) {
        let mut edited = original.clone();
        if self.tab == DetailTab::Information {
          edited.name = plan.text("appName", &original.name, input(&forms.name));
          edited.subtitle = plan.text("subtitle", &original.subtitle, input(&forms.subtitle));
        } else {
          edited.privacy_policy_url = plan.text("privacyPolicyUrl", &original.privacy_policy_url, input(&forms.privacy_url));
          edited.privacy_choices_url = plan.text("privacyChoicesUrl", &original.privacy_choices_url, input(&forms.privacy_choices));
        }
        if &edited != original {
          plan.info = Some((edited, original.clone()));
        }
      }
    }
    match self.tab {
      DetailTab::Information => {
        if let Some(original) = snapshot.version_localizations.iter().find(|item| item.locale == self.locale) {
          let edited = VersionLocalization {
            description: plan.text("description", &original.description, area(&forms.description)),
            keywords: plan.text("keywords", &original.keywords, input(&forms.keywords)),
            promotional_text: plan.text("promotionalText", &original.promotional_text, area(&forms.promotional)),
            support_url: plan.text("supportUrl", &original.support_url, input(&forms.support_url)),
            marketing_url: plan.text("marketingUrl", &original.marketing_url, input(&forms.marketing_url)),
            whats_new: if snapshot.version.as_ref().is_some_and(|version| version.supports_release_notes) {
              plan.text("whatsNewInThis", &original.whats_new, area(&forms.whats_new))
            } else {
              original.whats_new.clone()
            },
            ..original.clone()
          };
          if &edited != original {
            plan.listing = Some((edited, original.clone()));
          }
        }
      }
      DetailTab::Review => {
        if let Some(version) = &snapshot.version {
          let original = snapshot.review_detail.clone().unwrap_or_default();
          plan.boolean("signInRequiredTo", original.demo_account_required, forms.demo_required);
          let edited = ReviewDetail {
            id: original.id.clone(),
            contact_first_name: plan.text("firstName", &original.contact_first_name, input(&forms.first_name)),
            contact_last_name: plan.text("lastName", &original.contact_last_name, input(&forms.last_name)),
            contact_phone: plan.text("phone", &original.contact_phone, input(&forms.phone)),
            contact_email: plan.text("email", &original.contact_email, input(&forms.email)),
            demo_account_required: forms.demo_required,
            demo_account_name: plan.text("demoUserName", &original.demo_account_name, input(&forms.demo_name)),
            demo_account_password: plan.text_with_mask("demoPassword", &original.demo_account_password, input(&forms.demo_password), true),
            notes: plan.text("notes", &original.notes, area(&forms.notes)),
          };
          if edited != original {
            plan.review = Some((version.id.clone(), edited));
          }
        }
      }
      DetailTab::Privacy => {
        let original = self.privacy_declaration();
        let mut edited = self.privacy_draft(cx);
        edited.data_types = plan.text("dataTypesCollected", &original.data_types, edited.data_types);
        edited.third_party_sdks = plan.text("thirdPartySdks", &original.third_party_sdks, edited.third_party_sdks);
        for (label, before, after) in [
          ("iAnsweredAppPrivacy", original.confirmed_in_app_store_connect, edited.confirmed_in_app_store_connect),
          ("theAppCollectsData", original.collects_data, edited.collects_data),
          ("theAppTracksUsers", original.uses_tracking, edited.uses_tracking),
          ("theAppShowsThe", original.shows_tracking_prompt, edited.shows_tracking_prompt),
          ("peopleCanCreateAn", original.has_accounts, edited.has_accounts),
          ("accountsCanBeDeleted", original.account_deletion_in_app, edited.account_deletion_in_app),
          ("aReviewerMustSign", original.requires_login_to_review, edited.requires_login_to_review),
        ] {
          plan.boolean(label, before, after);
        }
        if edited != original {
          plan.privacy = Some(edited);
        }
      }
      _ => {}
    }
    plan
  }

  pub(crate) fn confirm_save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let plan = Arc::new(self.save_plan(cx, true));
    let Some(app) = self.selected.clone() else { return };
    if plan.changes.is_empty() || self.saving || self.store_write_busy || self.settings_locked {
      return;
    }
    let (tab, locale, language) = (self.tab, self.locale.clone(), self.settings.language);
    let workspace = cx.weak_entity();
    window.open_dialog(cx, move |dialog, _, cx| {
      let (workspace, app, locale, plan) = (workspace.clone(), app.clone(), locale.clone(), plan.clone());
      let c = colors(cx);
      let body = v_flex().id("save-change-list").w_full().max_h_96().gap_2().overflow_y_scrollbar().children(plan.changes.iter().map(|change| {
        let value = |text: &str, after| match change.kind {
          _ if text.is_empty() => tr(language, "saveValueEmpty"),
          ValueKind::Masked if after => tr(language, "saveValuePasswordChanged"),
          ValueKind::Masked => tr(language, "saveValueMasked"),
          ValueKind::Boolean => tr(language, if text == "saveValueYes" { "saveValueYes" } else { "saveValueNo" }),
          ValueKind::Text => SharedString::from(text.to_owned()),
        };
        v_flex()
          .gap_2()
          .p_2()
          .rounded_md()
          .border_1()
          .border_color(c.border)
          .child(div().font_weight(FontWeight::SEMIBOLD).child(tr(language, change.label)))
          .child(
            h_flex()
              .gap_2()
              .items_start()
              .child(div().flex_1().min_w_0().text_sm().text_color(c.text_secondary).child(value(&change.before, false)))
              .child(Icon::new(IconName::ArrowRight).small().text_color(c.text_secondary))
              .child(div().flex_1().min_w_0().text_sm().child(value(&change.after, true))),
          )
      }));
      dialog
        .width(SAVE_PREVIEW_WIDTH)
        .title(tr(language, "reviewSaveChanges"))
        .child(
          v_flex()
            .gap_2()
            .child(
              h_flex()
                .gap_2()
                .px_2()
                .text_sm()
                .text_color(c.text_secondary)
                .child(div().flex_1().child(tr(language, "saveOldValue")))
                .child(div().size_4())
                .child(div().flex_1().child(tr(language, "saveNewValue"))),
            )
            .child(body),
        )
        .footer(
          h_flex()
            .gap_2()
            .justify_end()
            .child(Button::new("cancel-save-preview").outline().label(tr(language, "cancel")).on_click(|_, window, cx| window.close_dialog(cx)))
            .child(Button::new("confirm-save-preview").primary().label(tr(language, "save")).on_click(move |_, window, cx| {
              window.close_dialog(cx);
              workspace
                .update(cx, |this, cx| {
                  if this.selected.as_ref() != Some(&app)
                    || this.tab != tab
                    || this.locale != locale
                    || this.saving
                    || this.store_write_busy
                    || this.settings_locked
                  {
                    return;
                  }
                  let current = this.save_plan(cx, true);
                  if current != *plan {
                    Self::warning(window, cx, tr(this.settings.language, "savePreviewChanged"));
                    return;
                  }
                  this.apply_save_plan(&app, current, window, cx);
                })
                .ok();
            })),
        )
    });
  }

  fn apply_save_plan(&mut self, app: &str, mut plan: SavePlan, window: &mut Window, cx: &mut Context<Self>) {
    let local_saved = plan.privacy.is_some();
    if let Some(privacy) = plan.privacy.take() {
      let mut settings = self.settings.clone();
      settings.privacy.insert(app.to_owned(), privacy.clone());
      if let Err(error) = settings.save(&self.data_dir.join("settings.json")) {
        Self::error(window, cx, trf(self.settings.language, "couldNotSaveSettings", &[&error.to_string()]));
        return;
      }
      self.settings = settings;
      self.forms.privacy = privacy;
      self.recompute_report();
      if !plan.has_remote_changes() {
        Self::success(window, cx, tr(self.settings.language, "savePrivacyLocalSuccess"));
        cx.notify();
      }
    }
    if !plan.has_remote_changes() {
      if let Some(history) = plan.history.filter(|_| local_saved) {
        self.record_local_history(history, window, cx);
      }
      return;
    }
    self.save_remote_fields(app.to_owned(), plan, local_saved, window, cx);
  }

  /// Records each successful write, so retries preview only the fields that still need saving.
  fn save_remote_fields(&mut self, app: String, plan: SavePlan, local_saved: bool, window: &mut Window, cx: &mut Context<Self>) {
    let Some(client) = self.client.clone() else { return };
    let history = self.form_history();
    let secrets = self.secrets.clone();
    let tab = self.tab;
    self.saving = true;
    if let Some(data) = self.app_data.get_mut(&app) {
      data.snapshot_task = None;
    }
    self.tasks.save = Some(cx.spawn_in(window, async move |this, cx| {
      let worker = client.clone();
      let (saved, result, history_result) = cx
        .background_spawn(async move {
          let SavePlan { info, listing, review, history: draft, .. } = plan;
          let mut saved = SavedFields::default();
          let result = (|| {
            if let Some((edited, original)) = info {
              worker.update_info_localization(&edited, &original)?;
              saved.info = Some(edited);
            }
            if let Some((edited, original)) = listing {
              worker.update_version_localization(&edited, &original)?;
              saved.listing = Some(edited);
            }
            if let Some((version, mut detail)) = review {
              let response = worker.save_review_detail(&version, &detail)?;
              detail.id = response.id.or(detail.id);
              saved.review = Some((version, detail));
            }
            Ok(())
          })();
          let history_result = history.zip(draft).map(|(history, draft)| {
            draft.save(&history, secrets.as_ref(), tab, [saved.info.is_some(), saved.listing.is_some(), saved.review.is_some(), local_saved])
          });
          (saved, result, history_result)
        })
        .await;
      this
        .update_in(cx, |this, window, cx| {
          this.saving = false;
          if let Some(Err(error)) = history_result {
            Self::warning(window, cx, trf(this.settings.language, "historyRecordFailed", &[&error]));
          }
          if !this.client.as_ref().is_some_and(|current| Arc::ptr_eq(current, &client)) {
            return;
          }
          if this.require_apple_sign_in(&result, window, cx) {
            return;
          }
          if let Some(data) = this.app_data.get_mut(&app) {
            if let Load::Ready(snapshot) = &mut data.snapshot {
              if let Some(edited) = saved.info {
                if let Some(original) = snapshot.info_localizations.iter_mut().find(|item| item.id == edited.id) {
                  *original = edited;
                }
              }
              if let Some(edited) = saved.listing {
                if let Some(original) = snapshot.version_localizations.iter_mut().find(|item| item.id == edited.id) {
                  *original = edited;
                }
              }
              if let Some((version, detail)) = saved.review {
                if snapshot.version.as_ref().is_some_and(|current| current.id == version) {
                  snapshot.review_detail = Some(detail);
                }
              }
            }
          }
          if this.selected.as_ref() == Some(&app) {
            this.recompute_report();
          }
          match result {
            Ok(()) => Self::success(window, cx, tr(this.settings.language, "savedToAppStore")),
            Err(error) => Self::error(window, cx, error.to_string()),
          }
          cx.notify();
        })
        .ok();
    }));
    cx.notify();
  }
}
