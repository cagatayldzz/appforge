//! "Fix with AI": confirms what is sent, asks the AI to rewrite the fields of one finding, and puts the result in the forms as an unsaved draft.

use crate::{
  forms::{read_area, read_input, set_area},
  i18n::{tr, trf},
  listing_ai::field_label,
  workspace::{DetailTab, Workspace},
};
use appforge_ai::Client;
use appforge_compliance::{can_fix, fix_finding, Finding, FixField, ListingField, ListingFields};
use appforge_storage::SecretKey;
use gpui_kit::{
  component::{
    button::{Button, ButtonVariants as _},
    h_flex, v_flex, WindowExt as _,
  },
  *,
};

/// Whether the button is offered for `finding`.
pub(crate) fn fixable(finding: &Finding) -> bool {
  can_fix(finding.category)
}

/// The page a fix lands on: the review notes, the name and the subtitle, or the listing texts.
fn page_for_field(field: FixField) -> DetailTab {
  match field {
    FixField::ReviewNotes => DetailTab::Review,
    FixField::Listing(ListingField::Name | ListingField::Subtitle) => DetailTab::Information,
    FixField::Listing(_) => DetailTab::ProductPage,
  }
}

impl Workspace {
  fn fix_current_value(&self, field: FixField, cx: &App) -> String {
    match field {
      FixField::Listing(field) => self.field_value(field, cx),
      FixField::ReviewNotes => read_area(&self.forms.notes, cx),
    }
  }

  fn fix_label(field: FixField) -> &'static str {
    match field {
      FixField::Listing(field) => field_label(field),
      FixField::ReviewNotes => "notes",
    }
  }

  /// Asks before anything is sent: what goes to the provider and that the result is only a draft.
  pub(crate) fn confirm_finding_fix(&self, finding: Finding, window: &mut Window, cx: &mut Context<Self>) {
    if self.fixing.is_some() || self.selected_app().is_none() {
      return;
    }
    let l = self.settings.language;
    let provider = self.settings.ai.provider;
    if !self.has_key.get(&provider).copied().unwrap_or(false) {
      self.ask_to_connect_ai(provider, window, cx);
      return;
    }
    let model = self.model_in_use().unwrap_or_default();
    let page = tr(l, crate::analysis::finding_page(&finding).label());
    let with_project = self.selected_app().is_some_and(|app| self.settings.code_folders.get(&app.id).is_some_and(|folder| folder.is_dir()));
    let workspace = cx.entity();
    let c = crate::theme::colors(cx);
    window.open_dialog(cx, move |dialog, _, _| {
      let (workspace, finding, confirmed) = (workspace.clone(), finding.clone(), finding.clone());
      let mut sent = vec![tr(l, "fixSendsFinding"), tr(l, "fixSendsTexts")];
      if with_project {
        sent.push(tr(l, "projectAnalysisSendsProject"));
      }
      let footer = h_flex()
        .w_full()
        .justify_end()
        .gap_2()
        .child(Button::new("cancel-finding-fix").label(tr(l, "cancel")).outline().on_click(|_, window, cx| window.close_dialog(cx)))
        .child(Button::new("confirm-finding-fix").label(tr(l, "fixWithAi")).primary().on_click(move |_, window, cx| {
          window.close_dialog(cx);
          let finding = confirmed.clone();
          workspace.update(cx, |this, cx| this.run_finding_fix(finding, window, cx));
        }));
      dialog
        .title(tr(l, "fixWithAi"))
        .child(
          v_flex()
            .gap_3()
            .child(div().font_weight(FontWeight::SEMIBOLD).child(crate::i18n::translated_text(l, &finding.title)))
            .child(div().text_color(c.text_secondary).child(trf(l, "fixProvider", &[provider.label(), &model])))
            .child(v_flex().gap_1().child(div().font_weight(FontWeight::MEDIUM).child(tr(l, "projectAnalysisSendsHeading"))).children(sent))
            .child(div().text_color(c.text_secondary).child(trf(l, "fixDraftOnly", &[&page])))
            .child(div().text_color(c.text_secondary).child(tr(l, "projectAnalysisEffect"))),
        )
        .footer(footer)
    });
  }

  fn run_finding_fix(&mut self, finding: Finding, window: &mut Window, cx: &mut Context<Self>) {
    if self.fixing.is_some() {
      return;
    }
    let Some(app) = self.selected_app() else { return };
    let l = self.settings.language;
    let provider = self.settings.ai.provider;
    let model = self.settings.ai.model(provider).to_owned();
    let used = model.clone();
    let secrets = self.secrets.clone();
    let locale = self.locale.clone();
    let folder = self.settings.code_folders.get(&app.id).cloned().filter(|folder| folder.is_dir());
    let current = ListingFields {
      name: read_input(&self.forms.name, cx),
      subtitle: read_input(&self.forms.subtitle, cx),
      description: read_area(&self.forms.description, cx),
      keywords: read_input(&self.forms.keywords, cx),
      promotional_text: read_area(&self.forms.promotional, cx),
      whats_new: read_area(&self.forms.whats_new, cx),
    };
    let notes = read_area(&self.forms.notes, cx);
    let sent = (current.clone(), notes.clone());
    let (target_app, target_locale, session) = (app.id, locale.clone(), self.client.clone());
    self.fixing = Some(format!("{}-{}", finding.id, finding.title));
    self.tasks.fix = Some(cx.spawn_in(window, async move |this, cx| {
      let result = cx
        .background_spawn(async move {
          let key = secrets
            .get(SecretKey::ApiKey(provider))
            .map_err(|error| error.to_string())?
            .ok_or_else(|| trf(l, "addYourApiKey", &[provider.label()]).to_string())?;
          let project = match folder {
            Some(folder) => appforge_project::summarize_for_review(&folder).map(|summary| summary.text).unwrap_or_default(),
            None => String::new(),
          };
          fix_finding(&Client::default(), provider, &key, &model, &finding, &current, &notes, &locale, &project)
        })
        .await;
      this
        .update_in(cx, |this, window, cx| {
          this.fixing = None;
          match result {
            Ok((fix, usage)) => {
              this.record_usage(provider, &used, usage, window, cx);
              let same_session =
                this.client.as_ref().zip(session.as_ref()).is_some_and(|(current, original)| std::sync::Arc::ptr_eq(current, original));
              if !same_session || this.selected.as_ref() != Some(&target_app) || this.locale != target_locale {
                Self::warning(window, cx, tr(l, "theDraftWasSkipped"));
              } else if fix.changes.is_empty() {
                let reason = if fix.summary.is_empty() { tr(l, "fixNothingChanged") } else { fix.summary.clone().into() };
                Self::warning(window, cx, reason);
              } else {
                this.apply_fix(&fix.changes, &sent, l, window, cx);
              }
            }
            Err(error) => Self::error(window, cx, error),
          }
          cx.notify();
        })
        .ok();
    }));
    cx.notify();
  }

  /// Puts the new texts in the forms. A field the user edited while the AI worked is left alone.
  fn apply_fix(
    &mut self,
    changes: &[(FixField, String)],
    sent: &(ListingFields, String),
    l: appforge_storage::Language,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    let mut applied = Vec::new();
    let mut applied_fields = Vec::new();
    for (field, value) in changes {
      let before = match field {
        FixField::Listing(field) => crate::listing_ai::listing_value(&sent.0, *field),
        FixField::ReviewNotes => sent.1.clone(),
      };
      if self.fix_current_value(*field, cx).trim() != before.trim() {
        continue;
      }
      match field {
        FixField::Listing(field) => self.apply_field_draft(*field, value, window, cx),
        FixField::ReviewNotes => set_area(&self.forms.notes, value, window, cx),
      }
      applied.push(tr(l, Self::fix_label(*field)).to_string());
      applied_fields.push(*field);
    }
    if applied.is_empty() {
      Self::warning(window, cx, tr(l, "theDraftWasSkipped"));
      return;
    }
    if let Some((field, _)) = changes.iter().find(|(field, _)| applied_fields.contains(field)) {
      self.jump_to(page_for_field(*field), window, cx);
    }
    Self::success(window, cx, trf(l, "fixApplied", &[&applied.join(", ")]));
  }
}
