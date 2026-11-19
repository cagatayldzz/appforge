//! The button that writes the store listing with an AI, from the app's source folder.

use crate::{
  forms::{read_area, read_input, set_area, set_input},
  i18n::{tr, trf},
  workspace::{model_label, SettingsPage, Workspace},
};
use appforge_ai::{Client, Provider};
use appforge_compliance::{draft_listing_field, ListingField, ListingFields};
use appforge_storage::SecretKey;
use gpui_kit::{
  component::{
    button::{Button, ButtonVariants as _},
    h_flex,
    notification::Notification,
    v_flex, Disableable as _, Icon, IconName, Sizable as _, WindowExt as _,
  },
  *,
};

/// The id of the notice that says the AI is not connected, so pressing the button again does not pile them up.
struct AiNotConnected;

impl Workspace {
  pub(crate) fn field_generate_button(&self, field: ListingField, cx: &mut Context<Self>) -> Button {
    let tooltip = trf(self.settings.language, "Generate {}", &[&tr(self.settings.language, field_label(field))]);
    Button::new(SharedString::from(format!("generate-{}", field.key())))
      .icon(Icon::new(IconName::Bot))
      .ghost()
      .xsmall()
      .loading(self.drafting == Some(field))
      .disabled(self.drafting.is_some())
      .tooltip(tooltip)
      .on_click(cx.listener(move |this, _, window, cx| this.confirm_field_generation(field, window, cx)))
  }

  pub(crate) fn confirm_field_generation(&self, field: ListingField, window: &mut Window, cx: &mut Context<Self>) {
    let Some(app) = self.selected_app() else { return };
    let l = self.settings.language;
    let locale: SharedString = self.locale.clone().into();
    let title = trf(l, "Generate {}", &[&tr(l, field_label(field))]);
    let model = self.model_in_use().map(SharedString::from);
    let workspace = cx.entity();
    window.open_dialog(cx, move |dialog, _, _| {
      let workspace = workspace.clone();
      let app_id = app.id.clone();
      let target_locale = locale.clone();
      let footer = h_flex()
        .w_full()
        .justify_end()
        .gap_2()
        .child(Button::new("cancel-field-generation").label(tr(l, "Cancel")).outline().on_click(|_, window, cx| window.close_dialog(cx)))
        .child(Button::new("confirm-field-generation").label(tr(l, "Generate")).primary().on_click(move |_, window, cx| {
          window.close_dialog(cx);
          workspace.update(cx, |this, cx| {
            if this.selected.as_ref() == Some(&app_id) && this.locale == target_locale.as_ref() {
              this.generate_listing(field, window, cx);
            }
          });
        }));
      dialog
        .title(title.clone())
        .child(
          v_flex()
            .gap_2()
            .child(format!("{} · {locale}", app.name))
            .children(model.clone())
            .child(tr(l, "Generate a draft for this field using the listing and project summary.")),
        )
        .footer(footer)
    });
  }

  fn generate_listing(&mut self, field: ListingField, window: &mut Window, cx: &mut Context<Self>) {
    if self.drafting.is_some() {
      return;
    }
    let Some(app) = self.selected_app() else { return };
    let provider = self.settings.ai.provider;
    if !self.has_key.get(&provider).copied().unwrap_or(false) {
      self.ask_to_connect_ai(provider, window, cx);
      return;
    }
    let Some(folder) = self.settings.code_folders.get(&app.id).cloned().filter(|folder| folder.is_dir()) else {
      self.ask_for_code_folder(app.id, app.name, field, window, cx);
      return;
    };
    let l = self.settings.language;
    let model = self.settings.ai.model(provider).to_owned();
    let secrets = self.secrets.clone();
    let locale = self.locale.clone();
    let current = ListingFields {
      name: read_input(&self.forms.name, cx),
      subtitle: read_input(&self.forms.subtitle, cx),
      description: read_area(&self.forms.description, cx),
      keywords: read_input(&self.forms.keywords, cx),
      promotional_text: read_area(&self.forms.promotional, cx),
      whats_new: read_area(&self.forms.whats_new, cx),
    };
    let original = self.field_value(field, cx);
    let target_app = app.id;
    let target_locale = locale.clone();
    let session = self.client.clone();
    self.drafting = Some(field);
    self.tasks.draft = Some(cx.spawn_in(window, async move |this, cx| {
      let used = model.clone();
      let result = cx
        .background_spawn(async move {
          let key = secrets
            .get(SecretKey::ApiKey(provider))
            .map_err(|error| error.to_string())?
            .ok_or_else(|| trf(l, "Add your {} API key in Settings first.", &[provider.label()]).to_string())?;
          let summary = appforge_project::summarize(&folder)
            .map_err(|error| trf(l, "Could not read the project folder: {}", &[&error.to_string()]).to_string())?;
          draft_listing_field(&Client::default(), provider, &key, &model, &current, &locale, &summary.text, field)
        })
        .await;
      this
        .update_in(cx, |this, window, cx| {
          this.drafting = None;
          match result {
            Ok((draft, usage)) => {
              this.record_usage(provider, &used, usage, window, cx);
              let same_session =
                this.client.as_ref().zip(session.as_ref()).is_some_and(|(current, original)| std::sync::Arc::ptr_eq(current, original));
              if !same_session
                || this.selected.as_ref() != Some(&target_app)
                || this.locale != target_locale
                || this.field_value(field, cx) != original
              {
                Self::warning(window, cx, tr(l, "The draft was skipped because the app, language, or field changed."));
                cx.notify();
                return;
              }
              if draft.is_empty() || draft == original {
                if original.is_empty() {
                  Self::warning(window, cx, tr(l, "No draft for this field. Add relevant details to the listing or project README and try again."));
                } else {
                  Self::success(window, cx, tr(l, "No alternative was generated. The current text was kept."));
                }
                cx.notify();
                return;
              }
              this.apply_field_draft(field, &draft, window, cx);
              let price = provider.model_info(&used).map(|info| format!("${:.4}", info.cost(usage))).unwrap_or_default();
              Self::success(
                window,
                cx,
                trf(l, "The draft is ready. Read it, then press Save. It used {} tokens (about {}).", &[&usage.total().to_string(), &price]),
              );
            }
            Err(error) => Self::error(window, cx, error),
          }
          cx.notify();
        })
        .ok();
    }));
    cx.notify();
  }

  fn field_value(&self, field: ListingField, cx: &App) -> String {
    match field {
      ListingField::Name => read_input(&self.forms.name, cx),
      ListingField::Subtitle => read_input(&self.forms.subtitle, cx),
      ListingField::Keywords => read_input(&self.forms.keywords, cx),
      ListingField::Description => read_area(&self.forms.description, cx),
      ListingField::PromotionalText => read_area(&self.forms.promotional, cx),
      ListingField::WhatsNew => read_area(&self.forms.whats_new, cx),
    }
  }

  fn apply_field_draft(&self, field: ListingField, draft: &str, window: &mut Window, cx: &mut Context<Self>) {
    match field {
      ListingField::Name => set_input(&self.forms.name, draft, window, cx),
      ListingField::Subtitle => set_input(&self.forms.subtitle, draft, window, cx),
      ListingField::Keywords => set_input(&self.forms.keywords, draft, window, cx),
      ListingField::Description => set_area(&self.forms.description, draft, window, cx),
      ListingField::PromotionalText => set_area(&self.forms.promotional, draft, window, cx),
      ListingField::WhatsNew => set_area(&self.forms.whats_new, draft, window, cx),
    }
  }

  /// The AI has no API key yet: say so, and offer to open the page where it is added.
  fn ask_to_connect_ai(&mut self, provider: Provider, window: &mut Window, cx: &mut Context<Self>) {
    let l = self.settings.language;
    let workspace = cx.entity();
    let button_label = tr(l, "Open Settings");
    let notification = Notification::warning(trf(l, "Add a {} API key in Settings to write the listing with AI.", &[provider.label()]))
      .id::<AiNotConnected>()
      .title(tr(l, "AI is not connected"))
      .action(move |_, _, _| {
        let workspace = workspace.clone();
        Button::new("connect-ai").label(button_label.clone()).outline().small().on_click(move |_, window, cx| {
          workspace.update(cx, |this, cx| {
            this.show_settings = true;
            this.settings_page = SettingsPage::AiProvider;
            cx.notify();
          });
          window.remove_notification::<AiNotConnected>(cx);
        })
      });
    window.push_notification(notification, cx);
  }

  /// The app has no source folder yet: ask for it in a dialog, and carry on when it is chosen.
  fn ask_for_code_folder(&mut self, app_id: String, app_name: String, field: ListingField, window: &mut Window, cx: &mut Context<Self>) {
    let l = self.settings.language;
    let workspace = cx.entity();
    let text = trf(
      l,
      "AppForge reads the README and the project files of {} to write the listing. Choose the folder that holds its source code.",
      &[&app_name],
    );
    window.open_dialog(cx, move |dialog, _, _| {
      let (workspace, app_id) = (workspace.clone(), app_id.clone());
      // A plain dialog draws no buttons of its own, so the footer is built here.
      let footer = h_flex()
        .w_full()
        .justify_end()
        .gap_2()
        .child(Button::new("folder-cancel").label(tr(l, "Cancel")).outline().on_click(|_, window, cx| window.close_dialog(cx)))
        .child(Button::new("folder-choose").icon(Icon::new(IconName::FolderOpen)).label(tr(l, "Choose folder")).primary().on_click(
          move |_, window, cx| {
            window.close_dialog(cx);
            let app_id = app_id.clone();
            workspace.update(cx, |this, cx| this.choose_code_folder(app_id, Some(field), window, cx));
          },
        ));
      dialog.title(tr(l, "Choose the project folder")).child(div().text_sm().child(text.clone())).footer(footer)
    });
  }

  /// The label of the model in use with its price, for the hint under the generate button.
  pub(crate) fn model_in_use(&self) -> Option<String> {
    let provider = self.settings.ai.provider;
    provider.model_info(self.settings.ai.model(provider)).map(model_label)
  }
}

pub(crate) fn field_label(field: ListingField) -> &'static str {
  match field {
    ListingField::Name => "App name",
    ListingField::Subtitle => "Subtitle",
    ListingField::Description => "Description",
    ListingField::Keywords => "Keywords",
    ListingField::PromotionalText => "Promotional text",
    ListingField::WhatsNew => "What's new in this version",
  }
}
