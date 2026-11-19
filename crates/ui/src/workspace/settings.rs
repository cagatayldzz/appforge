//! Saving settings, the remembered window place and the per-app code folders.

use super::*;

impl Workspace {
  /// Remembers the model chosen from the menu of `provider`.
  pub(super) fn set_model(&mut self, provider: Provider, model: &str, window: &mut Window, cx: &mut Context<Self>) {
    match provider {
      Provider::OpenAi => self.settings.ai.openai_model = model.to_owned(),
      Provider::Anthropic => self.settings.ai.anthropic_model = model.to_owned(),
    }
    self.save_settings(window, cx);
    cx.notify();
  }

  pub fn save_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    if self.settings_locked {
      Self::error(window, cx, tr(self.settings.language, "theSettingsFileCould"));
      return;
    }
    if let Err(error) = self.settings.save(&self.data_dir.join("settings.json")) {
      Self::error(window, cx, trf(self.settings.language, "couldNotSaveSettings", &[&error.to_string()]));
    }
  }

  /// Remembers the size and place of the window, so the next launch opens the same way. Dragging an edge changes the
  /// bounds many times a second, so the file is written once the window has been still for a moment.
  pub(super) fn remember_window(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let state = crate::window_state::state_of(window);
    if self.settings.window == Some(state) {
      return;
    }
    self.settings.window = Some(state);
    self.tasks.window = Some(cx.spawn_in(window, async move |this, cx| {
      cx.background_executor().timer(WINDOW_SAVE_DELAY).await;
      this
        .update(cx, |this, _| {
          // A settings file that could not be read is never overwritten; losing the window place is not worth a warning.
          if !this.settings_locked {
            if let Err(error) = this.settings.save(&this.data_dir.join("settings.json")) {
              eprintln!("Could not save the window place: {error}");
            }
          }
        })
        .ok();
    }));
  }

  /// Asks for the folder that holds the source code of an app and remembers it.
  pub fn choose_code_folder(&mut self, app_id: String, then_generate: Option<ListingField>, window: &mut Window, cx: &mut Context<Self>) {
    let receiver = cx.prompt_for_paths(PathPromptOptions {
      files: false,
      directories: true,
      multiple: false,
      prompt: Some(tr(self.settings.language, "chooseFolder")),
    });
    self.tasks.folder = Some(cx.spawn_in(window, async move |this, cx| {
      if let Ok(Ok(Some(paths))) = receiver.await {
        if let Some(path) = paths.into_iter().next() {
          this
            .update_in(cx, |this, window, cx| {
              this.settings.code_folders.insert(app_id.clone(), path);
              this.save_settings(window, cx);
              cx.notify();
              // Asked for from the generate button: carry on with what was being done.
              if this.selected.as_ref() == Some(&app_id) {
                if let Some(field) = then_generate {
                  this.confirm_field_generation(field, window, cx);
                }
              }
            })
            .ok();
        }
      }
    }));
  }

  pub fn clear_code_folder(&mut self, app_id: &str, window: &mut Window, cx: &mut Context<Self>) {
    if self.settings.code_folders.remove(app_id).is_some() {
      self.save_settings(window, cx);
      cx.notify();
    }
  }
}
