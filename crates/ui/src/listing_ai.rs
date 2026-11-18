//! The button that writes the store listing with an AI, from the app's source folder.

use crate::{
  forms::{read_area, read_input, set_area, set_input},
  i18n::{tr, trf},
  workspace::{model_label, SettingsPage, Workspace},
};
use appforge_ai::{Client, Provider};
use appforge_compliance::{draft_listing, ListingDraft, ListingFields};
use appforge_storage::SecretKey;
use gpui_kit::{
  component::{
    button::{Button, ButtonVariants as _},
    h_flex,
    notification::Notification,
    Icon, IconName, Sizable as _, WindowExt as _,
  },
  *,
};

/// The id of the notice that says the AI is not connected, so pressing the button again does not pile them up.
struct AiNotConnected;

impl Workspace {
  /// What the button does. It checks what is needed one thing at a time and says what is missing:
  /// an AI that is not connected is a notice with a way to Settings, a missing folder is a dialog that asks for it.
  pub(crate) fn generate_listing(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    if self.drafting {
      return;
    }
    let Some(app) = self.selected_app() else { return };
    let provider = self.settings.ai.provider;
    if !self.has_key.get(&provider).copied().unwrap_or(false) {
      self.ask_to_connect_ai(provider, window, cx);
      return;
    }
    let Some(folder) = self.settings.code_folders.get(&app.id).cloned().filter(|folder| folder.is_dir()) else {
      self.ask_for_code_folder(app.id, app.name, window, cx);
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
    self.drafting = true;
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
          draft_listing(&Client::default(), provider, &key, &model, &current, &locale, &summary.text)
        })
        .await;
      this
        .update_in(cx, |this, window, cx| {
          this.drafting = false;
          match result {
            Ok((draft, usage)) => {
              this.apply_draft(&draft, window, cx);
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

  /// Puts what the AI wrote in the fields. A field the AI left empty keeps what it had. Nothing is saved: the user reads it first.
  fn apply_draft(&mut self, draft: &ListingDraft, window: &mut Window, cx: &mut Context<Self>) {
    let forms = &self.forms;
    if !draft.subtitle.is_empty() {
      set_input(&forms.subtitle, &draft.subtitle, window, cx);
    }
    if !draft.description.is_empty() {
      set_area(&forms.description, &draft.description, window, cx);
    }
    if !draft.keywords.is_empty() {
      set_input(&forms.keywords, &draft.keywords, window, cx);
    }
    if !draft.promotional_text.is_empty() {
      set_area(&forms.promotional, &draft.promotional_text, window, cx);
    }
    if !draft.whats_new.is_empty() {
      set_area(&forms.whats_new, &draft.whats_new, window, cx);
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
  fn ask_for_code_folder(&mut self, app_id: String, app_name: String, window: &mut Window, cx: &mut Context<Self>) {
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
            workspace.update(cx, |this, cx| this.choose_code_folder(app_id, true, window, cx));
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
