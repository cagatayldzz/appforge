//! AI analysis, the playbook, provider and key settings, and the AI usage totals.

use super::*;
use gpui_kit::component::{
  button::{Button, ButtonVariants as _},
  h_flex, v_flex,
};

struct AiAnalysisNotice;

fn analysis_progress(message: SharedString, window: &mut Window, cx: &mut App) {
  window.push_notification(Notification::info(message).id::<AiAnalysisNotice>().autohide(false), cx);
}

impl Workspace {
  /// Counts what an AI request used and keeps the totals on disk.
  pub(crate) fn record_usage(&mut self, provider: Provider, model: &str, usage: appforge_ai::Usage, window: &mut Window, cx: &mut Context<Self>) {
    if usage.total() == 0 {
      return;
    }
    self.usage.record(provider, model, usage, now());
    self.save_usage(window, cx);
  }

  /// Writes the totals in the background. The whole ledger is written each time, so a newer write replaces an older one.
  pub(crate) fn save_usage(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    if self.usage_locked {
      return;
    }
    let (ledger, dir) = (self.usage.clone(), self.data_dir.clone());
    self.tasks.usage = Some(cx.spawn_in(window, async move |this, cx| {
      let result = cx.background_spawn(async move { ledger.save(&dir) }).await;
      if let Err(error) = result {
        this.update_in(cx, |this, window, cx| Self::warning(window, cx, trf(this.settings.language, "couldNotSaveThe", &[&error.to_string()]))).ok();
      }
    }));
  }

  /// Clears the AI usage totals.
  pub(crate) fn reset_usage(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    self.usage.reset();
    // A file that could not be read is replaced now: the user chose to start again.
    self.usage_locked = false;
    self.save_usage(window, cx);
    cx.notify();
  }

  pub(crate) fn confirm_ai(&mut self, require_project: bool, window: &mut Window, cx: &mut Context<Self>) {
    if self.tasks.ai.is_some() && matches!(self.ai, AiState::Running) {
      return;
    }
    let Some(app) = self.selected.clone() else { return };
    let Some(version) = self.ready_snapshot().map(|snapshot| snapshot.version.as_ref().map(|version| version.id.clone())) else { return };
    let l = self.settings.language;
    let provider = self.settings.ai.provider;
    let model = self.settings.ai.model(provider).to_owned();
    let folder = self.settings.code_folders.get(&app).cloned();
    let has_key = self.has_key.get(&provider).copied().unwrap_or(false);
    let workspace = cx.weak_entity();
    window.open_dialog(cx, move |dialog, _, _| {
      let (workspace, app, version, folder, model) = (workspace.clone(), app.clone(), version.clone(), folder.clone(), model.clone());
      let needs_folder = require_project && folder.is_none();
      let (message, action) = if !has_key {
        (trf(l, "addAApiKeyIn", &[provider.label()]), tr(l, "settings"))
      } else if needs_folder {
        (tr(l, "projectAnalysisNeedsFolder"), tr(l, "chooseFolder"))
      } else {
        (trf(l, if folder.is_some() { "projectAnalysisConsent" } else { "metadataAnalysisConsent" }, &[provider.label()]), tr(l, "aiAnalysis"))
      };
      dialog
        .title(tr(l, "aiAnalysis"))
        .child(v_flex().gap_2().child(message).children((has_key && !needs_folder).then(|| div().child(tr(l, "projectAnalysisEffect")))))
        .footer(
          h_flex()
            .gap_2()
            .justify_end()
            .child(Button::new("cancel-ai-analysis").outline().label(tr(l, "cancel")).on_click(|_, window, cx| window.close_dialog(cx)))
            .child(Button::new("confirm-ai-analysis").primary().label(action).on_click(move |_, window, cx| {
              window.close_dialog(cx);
              workspace
                .update(cx, |this, cx| {
                  if this.selected.as_ref() != Some(&app)
                    || this.ready_snapshot().map(|snapshot| snapshot.version.as_ref().map(|version| &version.id)) != Some(version.as_ref())
                    || this.settings.ai.provider != provider
                    || this.settings.ai.model(provider) != model
                  {
                    return;
                  }
                  if !has_key {
                    this.show_settings = true;
                    this.settings_page = SettingsPage::AiProvider;
                    cx.notify();
                  } else if needs_folder {
                    this.choose_code_folder(app.clone(), None, window, cx);
                  } else {
                    this.run_ai(folder.clone(), window, cx);
                  }
                })
                .ok();
            })),
        )
    });
  }

  fn run_ai(&mut self, folder: Option<PathBuf>, window: &mut Window, cx: &mut Context<Self>) {
    if matches!(self.ai, AiState::Running) {
      return;
    }
    let (Some(snapshot), Some(report)) = (self.ready_snapshot().cloned(), self.report.clone()) else { return };
    let version_id = snapshot.version.as_ref().map(|version| version.id.clone());
    let provider = self.settings.ai.provider;
    if !self.has_key.get(&provider).copied().unwrap_or(false) {
      Self::error(window, cx, trf(self.settings.language, "addYourApiKey", &[provider.label()]));
      return;
    }
    let language = self.settings.language;
    let missing_key = trf(language, "addYourApiKey", &[provider.label()]);
    let model = self.settings.ai.model(provider).to_owned();
    let used = model.clone();
    let declaration = self.privacy_declaration();
    let secrets = self.secrets.clone();
    let playbook = self.playbook.clone();
    let guidelines = self.guidelines.clone();
    let data_dir = self.data_dir.clone();
    self.ai = AiState::Running;
    analysis_progress(tr(language, "projectAnalysisPreparing"), window, cx);
    self.tasks.ai = Some(cx.spawn_in(window, async move |this, cx| {
      let prepared = cx
        .background_spawn(async move {
          let key = secrets.get(SecretKey::ApiKey(provider)).map_err(|error| error.to_string())?.ok_or_else(|| missing_key.to_string())?;
          // The rules are downloaded the first time they are needed.
          let playbook = if playbook.is_empty() { Playbook::refresh(&data_dir).unwrap_or(playbook) } else { playbook };
          let project = folder
            .map(|folder| appforge_project::summarize(&folder))
            .transpose()
            .map_err(|error| trf(language, "couldNotReadThe", &[&error.to_string()]).to_string())?;
          Ok((key, playbook, project))
        })
        .await;
      let result = match prepared {
        Ok((key, playbook, project)) => {
          if this
            .update_in(cx, |this, window, cx| {
              analysis_progress(trf(this.settings.language, "projectAnalysisWaiting", &[provider.label()]), window, cx);
            })
            .is_err()
          {
            return;
          }
          cx.background_spawn(async move {
            let client = appforge_ai::Client::default();
            let language_name = match language {
              Language::English => "English",
              Language::Turkish => "Turkish",
            };
            let context = Some((language_name, project.as_ref().map(|summary| summary.text.as_str())));
            analyze_project_with_ai(&client, provider, &key, &model, &snapshot, &declaration, &report, &playbook, &guidelines, context)
              .map(|(review, usage)| (review, playbook, usage))
          })
          .await
        }
        Err(error) => Err(error),
      };
      this
        .update_in(cx, |this, window, cx| {
          this.tasks.ai = None;
          if this.ready_snapshot().map(|snapshot| snapshot.version.as_ref().map(|version| &version.id)) != Some(version_id.as_ref()) {
            if let Ok((_, _, usage)) = result {
              this.record_usage(provider, &used, usage, window, cx);
            }
            this.reset_ai_analysis(window, cx);
            cx.notify();
            return;
          }
          match result {
            Ok((review, playbook, usage)) => {
              this.record_usage(provider, &used, usage, window, cx);
              this.playbook = playbook;
              let count = review.findings.iter().filter(|finding| !finding.fix.trim().is_empty()).count().to_string();
              this.ai = AiState::Done(review);
              window.push_notification(
                Notification::success(trf(this.settings.language, "projectAnalysisFinished", &[&count])).id::<AiAnalysisNotice>(),
                cx,
              );
            }
            Err(error) => {
              this.ai = AiState::Failed(error.clone().into());
              window
                .push_notification(Notification::error(trf(this.settings.language, "projectAnalysisFailed", &[&error])).id::<AiAnalysisNotice>(), cx);
            }
          }
          cx.notify();
        })
        .ok();
    }));
    cx.notify();
  }

  pub(super) fn reset_ai_analysis(&mut self, window: &mut Window, cx: &mut App) {
    self.tasks.ai = None;
    self.ai = AiState::Idle;
    window.remove_notification::<AiAnalysisNotice>(cx);
  }

  pub(super) fn refresh_playbook_if_stale(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let stale = self.playbook.fetched_at.is_none_or(|fetched| now().saturating_sub(fetched) > PLAYBOOK_MAX_AGE_SECONDS);
    if stale {
      self.refresh_playbook(false, window, cx);
    }
  }

  /// Downloads the playbook. `announce` shows a notification with the outcome (when the user asked for it).
  pub fn refresh_playbook(&mut self, announce: bool, window: &mut Window, cx: &mut Context<Self>) {
    if announce {
      self.refresh_guidelines(true, window, cx);
    }
    if self.playbook_busy {
      return;
    }
    let data_dir = self.data_dir.clone();
    self.playbook_busy = true;
    self.tasks.playbook = Some(cx.spawn_in(window, async move |this, cx| {
      let result = cx.background_spawn(async move { Playbook::refresh(&data_dir) }).await;
      this
        .update_in(cx, |this, window, cx| {
          this.playbook_busy = false;
          match result {
            Ok(playbook) => {
              this.playbook = playbook;
              this.playbook_error = None;
              if announce {
                Self::success(window, cx, tr(this.settings.language, "rulesUpdatedFromThe"));
              }
            }
            Err(error) => {
              this.playbook_error = Some(error.clone().into());
              if announce {
                Self::error(window, cx, error);
              }
            }
          }
          cx.notify();
        })
        .ok();
    }));
    cx.notify();
  }

  pub fn set_provider(&mut self, provider: Provider, window: &mut Window, cx: &mut Context<Self>) {
    self.settings.ai.provider = provider;
    self.save_settings(window, cx);
    cx.notify();
  }

  /// Saves the API key that was typed for `provider`.
  pub fn save_ai(&mut self, provider: Provider, window: &mut Window, cx: &mut Context<Self>) {
    let key_input = match provider {
      Provider::OpenAi => &self.settings_form.openai_key,
      Provider::Anthropic => &self.settings_form.anthropic_key,
    };
    let key = read_input(key_input, cx);
    if key.is_empty() {
      Self::success(window, cx, trf(self.settings.language, "settingsSaved", &[provider.label()]));
      cx.notify();
      return;
    }
    let secrets = self.secrets.clone();
    self.tasks.secrets = Some(cx.spawn_in(window, async move |this, cx| {
      let result = cx.background_spawn(async move { secrets.set(SecretKey::ApiKey(provider), &key) }).await;
      this
        .update_in(cx, |this, window, cx| {
          match result {
            Ok(()) => {
              this.has_key.insert(provider, true);
              let input = match provider {
                Provider::OpenAi => this.settings_form.openai_key.clone(),
                Provider::Anthropic => this.settings_form.anthropic_key.clone(),
              };
              set_input(&input, "", window, cx);
              Self::success(window, cx, trf(this.settings.language, "apiKeySavedIn", &[provider.label()]));
            }
            Err(error) => Self::error(window, cx, error.to_string()),
          }
          cx.notify();
        })
        .ok();
    }));
  }

  pub fn remove_key(&mut self, provider: Provider, window: &mut Window, cx: &mut Context<Self>) {
    let secrets = self.secrets.clone();
    self.tasks.secrets = Some(cx.spawn_in(window, async move |this, cx| {
      let result = cx.background_spawn(async move { secrets.delete(SecretKey::ApiKey(provider)) }).await;
      this
        .update_in(cx, |this, window, cx| {
          match result {
            Ok(()) => {
              this.has_key.insert(provider, false);
              Self::success(window, cx, trf(this.settings.language, "apiKeyRemoved", &[provider.label()]));
            }
            Err(error) => Self::error(window, cx, error.to_string()),
          }
          cx.notify();
        })
        .ok();
    }));
  }
}
