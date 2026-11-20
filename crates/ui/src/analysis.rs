//! The Analysis page: score, checklist, AI analysis and rules.

use crate::{
  i18n::{tr, trf},
  theme::{colors, FONT_SIZE_SM},
  widgets::{page_columns_with_primary, setting_row},
  workspace::{AiState, DetailTab, Workspace},
};
use appforge_compliance::{Category, Finding, Severity, Verdict, PLAYBOOK_REPOSITORY};
use appforge_storage::Language;
use gpui_kit::{
  component::{
    alert::Alert,
    button::{Button, ButtonVariants as _},
    h_flex,
    spinner::Spinner,
    v_flex, ActiveTheme, Icon, IconName, Selectable as _, Sizable as _,
  },
  *,
};
use std::time::{SystemTime, UNIX_EPOCH};

/// "3 days ago" for a time in seconds since the Unix epoch.
pub(crate) fn ago(l: Language, fetched_at: u64) -> SharedString {
  let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
  let seconds = now.saturating_sub(fetched_at);
  match seconds {
    0..=59 => tr(l, "justNow"),
    60..=3599 => trf(l, "minAgo", &[&(seconds / 60).to_string()]),
    3600..=86_399 => trf(l, "hoursAgo", &[&(seconds / 3600).to_string()]),
    _ => trf(l, "daysAgo", &[&(seconds / 86_400).to_string()]),
  }
}

/// "1 tip", "2 tips".
fn count(l: Language, number: usize, one: &'static str, many: &'static str) -> SharedString {
  trf(l, if number == 1 { one } else { many }, &[&number.to_string()])
}

/// The three parts of the page; one shows at a time so the page stays short.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum AnalysisTab {
  #[default]
  Checklist,
  Ai,
  Rules,
}

impl AnalysisTab {
  const ALL: [AnalysisTab; 3] = [Self::Checklist, Self::Ai, Self::Rules];

  fn label(self) -> &'static str {
    match self {
      Self::Checklist => "checklist",
      Self::Ai => "aiAnalysis",
      Self::Rules => "rules",
    }
  }
}

pub(crate) const SEVERITIES: [Severity; 3] = [Severity::Critical, Severity::Warning, Severity::Info];

pub(crate) fn severity_color(severity: Severity, c: &crate::theme::Colors) -> Hsla {
  match severity {
    Severity::Critical => c.danger,
    Severity::Warning => c.warning,
    Severity::Info => c.info,
  }
}

pub(crate) fn page_of(category: Category) -> DetailTab {
  match category {
    Category::AppInformation => DetailTab::Information,
    Category::AppReview => DetailTab::Review,
    Category::AppPrivacy => DetailTab::Privacy,
    Category::Distribution => DetailTab::Distribution,
    Category::ProductPage => DetailTab::ProductPage,
  }
}

impl Workspace {
  pub(crate) fn page_needs_attention(&self, tab: DetailTab, cx: &App) -> Option<bool> {
    let requirements = self.page_requirements(tab, cx);
    let rule_issue = self
      .report
      .as_ref()
      .is_some_and(|report| report.findings.iter().any(|finding| page_of(finding.category) == tab && finding.severity != Severity::Info));
    let ai_issue = matches!(&self.ai, AiState::Done(review) if review.findings.iter().any(|finding| page_of(finding.category) == tab && !finding.fix.trim().is_empty()));
    if rule_issue || ai_issue {
      return Some(true);
    }
    requirements.map(|checks| checks.iter().any(|(_, complete)| !complete))
  }

  fn missing_requirements(&self, cx: &App) -> Vec<(DetailTab, &'static str)> {
    DetailTab::ALL
      .iter()
      .flat_map(|&tab| self.page_requirements(tab, cx).into_iter().flatten().filter_map(move |(label, done)| (!done).then_some((tab, label))))
      .collect()
  }

  /// How many findings wait on each page, as buttons that open it: the answer to "where do I go".
  fn todo_by_page(&self, findings: &[&Finding], cx: &mut Context<Self>) -> Option<Div> {
    let l = self.settings.language;
    let c = colors(cx);
    let pages = [Category::AppInformation, Category::AppReview, Category::AppPrivacy];
    let buttons: Vec<_> = pages
      .into_iter()
      .filter_map(|category| {
        let number = findings.iter().filter(|finding| finding.category == category).count();
        (number > 0).then(|| {
          let tab = page_of(category);
          Button::new(SharedString::from(format!("todo-{}", category.label())))
            .label(format!("{} · {number}", tr(l, category.label())))
            .small()
            .outline()
            .on_click(cx.listener(move |this, _, window, cx| {
              this.jump_to(tab, window, cx);
            }))
        })
      })
      .collect();
    if buttons.is_empty() {
      return None;
    }
    Some(
      h_flex()
        .gap_2()
        .items_center()
        .flex_wrap()
        .child(div().text_size(FONT_SIZE_SM).text_color(c.text_secondary).child(tr(l, "goTo")))
        .children(buttons),
    )
  }

  pub(crate) fn render_analysis(&mut self, wide: bool, cx: &mut Context<Self>) -> impl IntoElement {
    let theme = cx.theme().clone();
    let c = colors(cx);
    let l = self.settings.language;
    let Some(report) = self.report.clone() else { return v_flex().into_any_element() };
    let (verdict, color) = match report.verdict() {
      Verdict::NotReady => (tr(l, "notReadyForReview"), theme.danger),
      Verdict::NeedsAttention => (tr(l, "almostReady"), theme.warning),
      Verdict::LooksReady => (tr(l, "looksReady"), theme.success),
    };
    let provider = self.settings.ai.provider;
    let has_key = self.has_key.get(&provider).copied().unwrap_or(false);
    let ai_findings: &[Finding] = match &self.ai {
      AiState::Done(review) => &review.findings,
      _ => &[],
    };
    let all: Vec<&Finding> = report.findings.iter().chain(ai_findings.iter()).collect();
    let todo = self.todo_by_page(&all, cx);
    let summary = v_flex()
      .w_full()
      .gap_3()
      .p_4()
      .rounded_lg()
      .bg(c.surface_alt)
      .child(
        h_flex()
          .gap_4()
          .items_center()
          .child(div().text_color(color).text_size(rems(2.5)).font_weight(FontWeight::BOLD).child(report.score().to_string()))
          .child(v_flex().gap_1().child(div().font_weight(FontWeight::SEMIBOLD).text_color(color).child(verdict)).child(
            div().text_sm().text_color(theme.muted_foreground).child(trf(
              l,
              "criticalSummary",
              &[
                &report.count(Severity::Critical).to_string(),
                &count(l, report.count(Severity::Warning), "warningOne", "warningMany"),
                &count(l, report.count(Severity::Info), "tipOne", "tipMany"),
              ],
            )),
          )),
      )
      .children(todo)
      .child(div().text_size(FONT_SIZE_SM).text_color(c.text_secondary).child(tr(l, "basedOnTheData")));
    let tabs = h_flex().gap_1().flex_wrap().children(AnalysisTab::ALL.map(|option| {
      let label = match option {
        AnalysisTab::Checklist => tr(l, option.label()).to_string(),
        AnalysisTab::Ai if !ai_findings.is_empty() => format!("{} · {}", tr(l, option.label()), ai_findings.len()),
        _ => tr(l, option.label()).to_string(),
      };
      Button::new(SharedString::from(format!("analysis-tab-{}", option.label())))
        .label(label)
        .small()
        .outline()
        .selected(self.analysis_tab == option)
        .on_click(cx.listener(move |this, _, _, cx| {
          this.analysis_tab = option;
          cx.notify();
        }))
    }));
    let content = match self.analysis_tab {
      AnalysisTab::Checklist => {
        let missing = self.missing_requirements(cx);
        let mut checklist = v_flex().gap_4();
        if !missing.is_empty() {
          checklist = checklist.child(v_flex().gap_2().child(div().font_weight(FontWeight::SEMIBOLD).child(tr(l, "missingRequirements"))).children(
            missing.into_iter().map(|(tab, label)| {
              h_flex().gap_2().flex_wrap().items_center().justify_between().child(div().child(tr(l, label))).child(
                Button::new(SharedString::from(format!("missing-{label}")))
                  .small()
                  .outline()
                  .label(tr(l, tab.label()))
                  .on_click(cx.listener(move |this, _, window, cx| this.jump_to(tab, window, cx))),
              )
            }),
          ));
        } else if report.findings.is_empty() && ai_findings.is_empty() {
          checklist = checklist.child(div().text_sm().text_color(theme.muted_foreground).child(tr(l, "noIssuesFoundIn")));
        }
        if !report.findings.is_empty() {
          checklist = checklist.child(self.finding_groups("rule", &report.findings, cx));
        }
        if !ai_findings.is_empty() {
          checklist = checklist.child(
            v_flex().gap_2().child(div().font_weight(FontWeight::SEMIBOLD).child(tr(l, "projectAnalysisTasks"))).child(self.finding_groups(
              "ai",
              ai_findings,
              cx,
            )),
          );
        }
        checklist.into_any_element()
      }
      AnalysisTab::Ai => self.render_ai_tab(ai_findings, provider, has_key, cx).into_any_element(),
      AnalysisTab::Rules => {
        let playbook_status: SharedString = match (self.playbook.fetched_at, self.playbook.is_empty()) {
          (_, true) => tr(l, "notDownloadedYet"),
          (Some(fetched), false) => trf(l, "updated", &[&ago(l, fetched)]),
          (None, false) => tr(l, "downloaded"),
        };
        setting_row(
          tr(l, "appStoreCompliancePlaybook"),
          Some(playbook_status),
          h_flex()
            .gap_2()
            .child(Button::new("open-playbook").label(tr(l, "view")).outline().small().on_click(|_, _, cx| cx.open_url(PLAYBOOK_REPOSITORY)))
            .child(
              Button::new("update-playbook")
                .label(tr(l, "updateRules"))
                .outline()
                .small()
                .loading(self.playbook_busy)
                .on_click(cx.listener(|this, _, window, cx| this.refresh_playbook(true, window, cx))),
            ),
          cx,
        )
        .into_any_element()
      }
    };
    let findings = v_flex().gap_5().child(tabs).child(content);
    if wide {
      page_columns_with_primary(true, 0.68, findings, summary).into_any_element()
    } else {
      v_flex().gap_5().child(summary).child(findings).into_any_element()
    }
  }

  fn render_ai_tab(&self, findings: &[Finding], provider: appforge_ai::Provider, has_key: bool, cx: &mut Context<Self>) -> Div {
    let theme = cx.theme().clone();
    let c = colors(cx);
    let l = self.settings.language;
    let hint: SharedString = if has_key { trf(l, "sendsTheListingText", &[provider.label()]) } else { trf(l, "addAApiKeyIn", &[provider.label()]) };
    let run = h_flex()
      .gap_3()
      .items_center()
      .child(
        Button::new("run-ai")
          .icon(Icon::new(IconName::Bot))
          .label(trf(l, "analyzeWith", &[provider.label()]))
          .primary()
          .loading(matches!(self.ai, AiState::Running(_)) || self.tasks.ai_dialog.is_some())
          .on_click(cx.listener(|this, _, window, cx| this.confirm_ai(false, window, cx))),
      )
      .child(div().flex_1().text_size(FONT_SIZE_SM).text_color(c.text_secondary).child(hint));
    let result = match &self.ai {
      AiState::Idle => v_flex(),
      AiState::Running(message) => {
        v_flex().child(h_flex().gap_2().items_center().text_sm().text_color(theme.muted_foreground).child(Spinner::new()).child(message.clone()))
      }
      AiState::Failed(message) => v_flex().child(Alert::error("ai-error", message.clone())),
      AiState::Done(review) => {
        let mut body = v_flex().gap_4();
        if !review.summary.is_empty() {
          body = body.child(
            v_flex()
              .gap_1()
              .p_3()
              .rounded_lg()
              .bg(c.surface_alt)
              .child(div().text_size(FONT_SIZE_SM).font_weight(FontWeight::SEMIBOLD).text_color(c.text_secondary).child(tr(l, "summary")))
              .child(div().child(review.summary.clone())),
          );
        }
        if findings.is_empty() {
          body = body.child(div().text_sm().text_color(theme.muted_foreground).child(tr(l, "theAiFoundNothing")));
        } else {
          body = body.child(self.finding_groups("ai", findings, cx));
        }
        body
      }
    };
    v_flex().gap_4().child(run).child(result)
  }
}
