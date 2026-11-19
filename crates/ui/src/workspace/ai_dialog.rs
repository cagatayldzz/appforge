//! Offers saved AI reviews, local checks and explicit provider reruns.

use super::*;
use appforge_compliance::{AiCache, CachedReview};
use gpui_kit::component::{
  button::{Button, ButtonVariants as _},
  h_flex, v_flex,
};

#[derive(Clone, PartialEq, Eq)]
struct ReviewScope {
  issuer: String,
  app: String,
  version: Option<String>,
  language: Language,
  folder: Option<PathBuf>,
}

impl ReviewScope {
  fn cache(&self, data_dir: &std::path::Path, project: bool) -> AiCache {
    AiCache::new(data_dir, &self.issuer, &self.app, self.version.as_deref(), self.language, project)
  }
}

impl Workspace {
  fn review_scope(&self) -> Option<ReviewScope> {
    let issuer = self.settings.apple.as_ref()?.issuer_id.clone();
    let app = self.selected.clone()?;
    let version = self.ready_snapshot()?.version.as_ref().map(|version| version.id.clone());
    let folder = self.settings.code_folders.get(&app).cloned();
    Some(ReviewScope { issuer, app, version, language: self.settings.language, folder })
  }

  pub(super) fn analysis_cache(&self, project: bool) -> Option<AiCache> {
    Some(self.review_scope()?.cache(&self.data_dir, project))
  }

  pub(crate) fn confirm_ai(&mut self, require_project: bool, window: &mut Window, cx: &mut Context<Self>) {
    if self.tasks.ai_dialog.is_some() || matches!(self.ai, AiState::Running(_)) {
      return;
    }
    let Some(scope) = self.review_scope() else { return };
    let cache = scope.cache(&self.data_dir, require_project || scope.folder.is_some());
    self.tasks.ai_dialog = Some(cx.spawn_in(window, async move |this, cx| {
      let result = cx.background_spawn(async move { cache.load().map_err(|error| error.to_string()) }).await;
      this
        .update_in(cx, |this, window, cx| {
          this.tasks.ai_dialog = None;
          cx.notify();
          if this.review_scope().as_ref() != Some(&scope) {
            return;
          }
          let cached = match result {
            Ok(cached) => cached,
            Err(error) => {
              Self::warning(window, cx, trf(this.settings.language, "analysisCacheReadFailed", &[&error]));
              None
            }
          };
          this.show_analysis_options(scope, require_project, cached, window, cx);
        })
        .ok();
    }));
    cx.notify();
  }

  fn show_analysis_options(
    &self,
    scope: ReviewScope,
    require_project: bool,
    cached: Option<CachedReview>,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    let l = scope.language;
    let provider = self.settings.ai.provider;
    let model = self.settings.ai.model(provider).to_owned();
    let has_key = self.has_key.get(&provider).copied().unwrap_or(false);
    let workspace = cx.weak_entity();
    window.open_dialog(cx, move |dialog, _, _| {
      let scope_for_run = scope.clone();
      let workspace_for_run = workspace.clone();
      let model = model.clone();
      let needs_folder = require_project && scope.folder.is_none();
      let (message, action) = if !has_key {
        (trf(l, "addAApiKeyIn", &[provider.label()]), tr(l, "settings"))
      } else if needs_folder {
        (tr(l, "projectAnalysisNeedsFolder"), tr(l, "chooseFolder"))
      } else {
        (
          trf(l, if scope.folder.is_some() { "projectAnalysisConsent" } else { "metadataAnalysisConsent" }, &[provider.label()]),
          tr(l, if cached.is_some() { "rerunAnalysis" } else { "aiAnalysis" }),
        )
      };
      let saved = cached.as_ref().map(|entry| {
        let time = i64::try_from(entry.finished_at).ok().and_then(|time| chrono::DateTime::from_timestamp(time, 0));
        let when = time.map_or_else(|| entry.finished_at.to_string(), |time| time.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M").to_string());
        let count = entry.review.findings.len().to_string();
        let scope = scope.clone();
        let workspace = workspace.clone();
        let entry = entry.clone();
        v_flex()
          .gap_2()
          .child(div().font_weight(FontWeight::SEMIBOLD).child(trf(l, "lastAnalysisAt", &[&when])))
          .child(div().text_sm().child(trf(l, "cachedAnalysisSummary", &[entry.provider.label(), &entry.model, &count])))
          .child(h_flex().child(Button::new("apply-cached-analysis").outline().label(tr(l, "loadAnalysisResults")).on_click(move |_, window, cx| {
            window.close_dialog(cx);
            workspace
              .update(cx, |this, cx| {
                if this.review_scope().as_ref() == Some(&scope) && !matches!(this.ai, AiState::Running(_)) {
                  this.ai = AiState::Done(entry.review.clone());
                  this.analysis_tab = crate::analysis::AnalysisTab::Checklist;
                  this.jump_to(DetailTab::Analysis, window, cx);
                }
              })
              .ok();
          })))
      });
      let quick = scope.folder.is_some().then(|| {
        let workspace = workspace.clone();
        let scope = scope.clone();
        Button::new("run-local-project-check").outline().label(tr(l, "quickLocalAnalysis")).on_click(move |_, window, cx| {
          window.close_dialog(cx);
          workspace.update(cx, |this, cx| this.run_local_analysis(scope.clone(), window, cx)).ok();
        })
      });
      dialog
        .title(tr(l, "aiAnalysis"))
        .child(
          v_flex()
            .gap_3()
            .children(saved)
            .child(message)
            .children((has_key && !needs_folder).then(|| div().text_sm().child(tr(l, "projectAnalysisEffect")))),
        )
        .footer(
          h_flex()
            .gap_2()
            .flex_wrap()
            .justify_end()
            .child(Button::new("cancel-ai-analysis").outline().label(tr(l, "cancel")).on_click(|_, window, cx| window.close_dialog(cx)))
            .children(quick)
            .child(Button::new("confirm-ai-analysis").primary().label(action).on_click(move |_, window, cx| {
              window.close_dialog(cx);
              workspace_for_run
                .update(cx, |this, cx| {
                  if this.review_scope().as_ref() != Some(&scope_for_run)
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
                    this.choose_code_folder(scope_for_run.app.clone(), None, window, cx);
                  } else {
                    this.run_ai(scope_for_run.folder.clone(), window, cx);
                  }
                })
                .ok();
            })),
        )
    });
  }

  fn run_local_analysis(&mut self, scope: ReviewScope, window: &mut Window, cx: &mut Context<Self>) {
    if self.tasks.ai_dialog.is_some() || self.review_scope().as_ref() != Some(&scope) {
      return;
    }
    let Some(folder) = scope.folder.clone() else { return };
    let (Some(snapshot), privacy) = (self.ready_snapshot().cloned(), self.privacy_declaration()) else { return };
    self.tasks.ai_dialog = Some(cx.spawn_in(window, async move |this, cx| {
      let result = cx
        .background_spawn(async move {
          let signals = appforge_project::quick_check(&folder)?;
          Ok::<_, std::io::Error>((signals, check(&snapshot, &privacy)))
        })
        .await;
      this
        .update_in(cx, |this, window, cx| {
          this.tasks.ai_dialog = None;
          cx.notify();
          if this.review_scope().as_ref() != Some(&scope) {
            return;
          }
          let l = scope.language;
          match result {
            Ok((signals, report)) => {
              this.report = Some(Arc::new(report));
              let signals: Vec<_> = signals
                .iter()
                .filter_map(|signal| match signal.as_str() {
                  "advertising" => Some(tr(l, "localCheckAdvertising")),
                  "tracking" => Some(tr(l, "localCheckTracking")),
                  "privacy-manifest" => Some(tr(l, "localCheckManifest")),
                  _ => None,
                })
                .collect();
              this.analysis_tab = crate::analysis::AnalysisTab::Checklist;
              this.jump_to(DetailTab::Analysis, window, cx);
              window.open_dialog(cx, move |dialog, _, _| {
                dialog.title(tr(l, "quickLocalAnalysis")).child(
                  v_flex()
                    .gap_2()
                    .child(tr(l, "localCheckScope"))
                    .children(signals.is_empty().then(|| div().child(tr(l, "localCheckNoSignals"))))
                    .children(signals.iter().cloned().map(|signal| div().child(signal))),
                )
              });
            }
            Err(error) => Self::error(window, cx, trf(l, "couldNotReadThe", &[&error.to_string()])),
          }
        })
        .ok();
    }));
    cx.notify();
  }
}
