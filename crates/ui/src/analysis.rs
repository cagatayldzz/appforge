use crate::{
  i18n::{tr, trf},
  theme::{colors, FONT_SIZE_SM},
  widgets::{setting_row, severity_badge},
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
    v_flex, ActiveTheme, Disableable as _, Icon, IconName, Selectable as _, Sizable as _,
  },
  *,
};
use std::time::{SystemTime, UNIX_EPOCH};

/// "3 days ago" for a time in seconds since the Unix epoch.
pub(crate) fn ago(l: Language, fetched_at: u64) -> SharedString {
  let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
  let seconds = now.saturating_sub(fetched_at);
  match seconds {
    0..=59 => tr(l, "just now"),
    60..=3599 => trf(l, "{} min ago", &[&(seconds / 60).to_string()]),
    3600..=86_399 => trf(l, "{} h ago", &[&(seconds / 3600).to_string()]),
    _ => trf(l, "{} days ago", &[&(seconds / 86_400).to_string()]),
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
      Self::Checklist => "Checklist",
      Self::Ai => "AI analysis",
      Self::Rules => "Rules",
    }
  }
}

const SEVERITIES: [Severity; 3] = [Severity::Critical, Severity::Warning, Severity::Info];

fn severity_color(severity: Severity, c: &crate::theme::Colors) -> Hsla {
  match severity {
    Severity::Critical => c.danger,
    Severity::Warning => c.warning,
    Severity::Info => c.info,
  }
}

fn page_of(category: Category) -> DetailTab {
  match category {
    Category::AppInformation => DetailTab::Information,
    Category::AppReview => DetailTab::Review,
    Category::AppPrivacy => DetailTab::Privacy,
  }
}

impl Workspace {
  /// Critical findings and warnings start open, tips closed; a click flips that for one finding.
  fn finding_open(&self, key: &str, severity: Severity) -> bool {
    (severity != Severity::Info) != self.toggled_findings.contains(key)
  }

  /// One finding: a header that opens and closes it, and, open, what is wrong, how to fix it and where.
  fn finding_row(&self, scope: &'static str, finding: &Finding, cx: &mut Context<Self>) -> impl IntoElement {
    let l = self.settings.language;
    let c = colors(cx);
    let key = format!("{scope}-{}-{}", finding.id, finding.title);
    let open = self.finding_open(&key, finding.severity);
    let tab = page_of(finding.category);
    let toggle_key = key.clone();
    let mut header = h_flex()
      .id(SharedString::from(format!("finding-{key}")))
      .w_full()
      .gap_2()
      .items_center()
      .cursor_pointer()
      .child(Icon::new(if open { IconName::ChevronDown } else { IconName::ChevronRight }).small().text_color(c.text_secondary))
      .child(severity_badge(finding.severity, l, cx))
      .child(div().flex_1().min_w_0().font_weight(FontWeight::SEMIBOLD).child(finding.title.clone()))
      .child(div().flex_shrink_0().text_size(FONT_SIZE_SM).text_color(c.text_secondary).child(tr(l, finding.category.label())))
      .on_click(cx.listener(move |this, _, _, cx| {
        if !this.toggled_findings.remove(&toggle_key) {
          this.toggled_findings.insert(toggle_key.clone());
        }
        cx.notify();
      }));
    if !finding.guideline.is_empty() {
      header =
        header.child(div().flex_shrink_0().text_size(FONT_SIZE_SM).text_color(c.text_secondary).child(trf(l, "Guideline {}", &[&finding.guideline])));
    }
    let mut card = v_flex().w_full().p_3().gap_3().rounded_lg().border_1().border_color(c.border).child(header);
    if open {
      let source = self.guidelines.rule(&finding.guideline).map(|rule| {
        let url = rule.url();
        Button::new(SharedString::from(format!("guideline-{key}")))
          .label(tr(l, "View Apple rule"))
          .small()
          .ghost()
          .on_click(move |_, _, cx| cx.open_url(&url))
      });
      card = card
        .child(div().pl_6().text_color(c.text_secondary).child(finding.detail.clone()))
        .child(
          v_flex()
            .ml_6()
            .p_3()
            .gap_1()
            .rounded_md()
            .bg(c.surface)
            .child(div().text_size(FONT_SIZE_SM).font_weight(FontWeight::SEMIBOLD).text_color(c.text_secondary).child(tr(l, "How to fix")))
            .child(div().child(finding.fix.clone())),
        )
        .child(
          h_flex()
            .pl_6()
            .gap_2()
            .child(
              Button::new(SharedString::from(format!("go-{key}")))
                .label(trf(l, "Open {}", &[&tr(l, finding.category.label())]))
                .small()
                .outline()
                .on_click(cx.listener(move |this, _, _, cx| {
                  this.tab = tab;
                  cx.notify();
                })),
            )
            .children(source),
        );
    }
    card
  }

  /// Findings under a heading per severity, the worst first.
  fn finding_groups(&self, scope: &'static str, findings: &[Finding], cx: &mut Context<Self>) -> Div {
    let l = self.settings.language;
    let c = colors(cx);
    let mut groups = v_flex().w_full().gap_5();
    for severity in SEVERITIES {
      let items: Vec<&Finding> = findings.iter().filter(|finding| finding.severity == severity).collect();
      if items.is_empty() {
        continue;
      }
      let heading = h_flex()
        .gap_2()
        .items_center()
        .child(div().size_2().rounded_full().bg(severity_color(severity, &c)))
        .child(div().font_weight(FontWeight::SEMIBOLD).child(format!("{} · {}", tr(l, severity.label()), items.len())));
      groups = groups.child(v_flex().gap_2().child(heading).children(items.into_iter().map(|finding| self.finding_row(scope, finding, cx))));
    }
    groups
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
        .child(div().text_size(FONT_SIZE_SM).text_color(c.text_secondary).child(tr(l, "Go to")))
        .children(buttons),
    )
  }

  pub(crate) fn render_analysis(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
    let theme = cx.theme().clone();
    let c = colors(cx);
    let l = self.settings.language;
    let Some(report) = self.report.clone() else { return v_flex().into_any_element() };
    let (verdict, color) = match report.verdict() {
      Verdict::NotReady => (tr(l, "Not ready for review"), theme.danger),
      Verdict::NeedsAttention => (tr(l, "Almost ready"), theme.warning),
      Verdict::LooksReady => (tr(l, "Looks ready"), theme.success),
    };
    let provider = self.settings.ai.provider;
    let has_key = self.has_key.get(&provider).copied().unwrap_or(false);
    let ai_findings: Vec<Finding> = match &self.ai {
      AiState::Done(review) => review.findings.clone(),
      _ => Vec::new(),
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
              "{} critical · {} · {}",
              &[
                &report.count(Severity::Critical).to_string(),
                &count(l, report.count(Severity::Warning), "{} warning", "{} warnings"),
                &count(l, report.count(Severity::Info), "{} tip", "{} tips"),
              ],
            )),
          )),
      )
      .children(todo)
      .child(div().text_size(FONT_SIZE_SM).text_color(c.text_secondary).child(tr(
        l,
        "Based on the data saved in App Store Connect and your App Privacy answers. Apple's reviewers also test the app itself, so no check can promise approval.",
      )));
    let tabs = h_flex().gap_1().children(AnalysisTab::ALL.map(|option| {
      let label = match option {
        AnalysisTab::Checklist => format!("{} · {}", tr(l, option.label()), report.findings.len()),
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
      AnalysisTab::Checklist if report.findings.is_empty() => {
        div().text_sm().text_color(theme.muted_foreground).child(tr(l, "No issues found in the saved data.")).into_any_element()
      }
      AnalysisTab::Checklist => self.finding_groups("rule", &report.findings, cx).into_any_element(),
      AnalysisTab::Ai => self.render_ai_tab(&ai_findings, provider, has_key, cx).into_any_element(),
      AnalysisTab::Rules => {
        let playbook_status: SharedString = match (self.playbook.fetched_at, self.playbook.is_empty()) {
          (_, true) => tr(l, "Not downloaded yet"),
          (Some(fetched), false) => trf(l, "Updated {}", &[&ago(l, fetched)]),
          (None, false) => tr(l, "Downloaded"),
        };
        setting_row(
          tr(l, "App Store Compliance Playbook"),
          Some(playbook_status),
          h_flex()
            .gap_2()
            .child(Button::new("open-playbook").label(tr(l, "View")).outline().small().on_click(|_, _, cx| cx.open_url(PLAYBOOK_REPOSITORY)))
            .child(
              Button::new("update-playbook")
                .label(tr(l, "Update rules"))
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
    v_flex().gap_5().child(summary).child(tabs).child(content).into_any_element()
  }

  fn render_ai_tab(&self, findings: &[Finding], provider: appforge_ai::Provider, has_key: bool, cx: &mut Context<Self>) -> Div {
    let theme = cx.theme().clone();
    let c = colors(cx);
    let l = self.settings.language;
    let hint: SharedString = if has_key {
      trf(l, "Sends the listing text, the review notes and your privacy answers to {} together with the rules below. Demo passwords and contact details are never sent.", &[provider.label()])
    } else {
      trf(l, "Add a {} API key in Settings to run the AI analysis.", &[provider.label()])
    };
    let run = h_flex()
      .gap_3()
      .items_center()
      .child(
        Button::new("run-ai")
          .icon(Icon::new(IconName::Bot))
          .label(trf(l, "Analyze with {}", &[provider.label()]))
          .primary()
          .loading(matches!(self.ai, AiState::Running))
          .disabled(!has_key)
          .on_click(cx.listener(|this, _, window, cx| this.run_ai(window, cx))),
      )
      .child(div().flex_1().text_size(FONT_SIZE_SM).text_color(c.text_secondary).child(hint));
    let result = match &self.ai {
      AiState::Idle => v_flex(),
      AiState::Running => {
        v_flex().child(h_flex().gap_2().items_center().text_sm().text_color(theme.muted_foreground).child(Spinner::new()).child(trf(
          l,
          "{} is reading your app and the rules…",
          &[provider.label()],
        )))
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
              .child(div().text_size(FONT_SIZE_SM).font_weight(FontWeight::SEMIBOLD).text_color(c.text_secondary).child(tr(l, "Summary")))
              .child(div().child(review.summary.clone())),
          );
        }
        if findings.is_empty() {
          body = body.child(div().text_sm().text_color(theme.muted_foreground).child(tr(l, "The AI found nothing more to add.")));
        } else {
          body = body.child(self.finding_groups("ai", findings, cx));
        }
        body
      }
    };
    v_flex().gap_4().child(run).child(result)
  }
}
