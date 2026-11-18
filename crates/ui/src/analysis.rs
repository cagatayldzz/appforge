use crate::{
  widgets::{finding_card, page_title},
  workspace::{AiState, Workspace},
};
use appforge_compliance::{Finding, Severity, Verdict, PLAYBOOK_REPOSITORY};
use gpui_kit::{
  component::{
    alert::Alert,
    button::{Button, ButtonVariants as _},
    h_flex,
    spinner::Spinner,
    v_flex, ActiveTheme, Disableable as _, Icon, IconName, Sizable as _,
  },
  *,
};
use std::time::{SystemTime, UNIX_EPOCH};

/// "3 days ago" for a time in seconds since the Unix epoch.
fn ago(fetched_at: u64) -> String {
  let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
  let seconds = now.saturating_sub(fetched_at);
  match seconds {
    0..=59 => "just now".into(),
    60..=3599 => format!("{} min ago", seconds / 60),
    3600..=86_399 => format!("{} h ago", seconds / 3600),
    _ => format!("{} days ago", seconds / 86_400),
  }
}

/// "1 tip", "2 tips".
fn count(number: usize, noun: &str) -> String {
  if number == 1 {
    format!("1 {noun}")
  } else {
    format!("{number} {noun}s")
  }
}

impl Workspace {
  fn finding_with_action(&self, finding: &Finding, cx: &mut Context<Self>) -> impl IntoElement {
    let tab = match finding.category {
      appforge_compliance::Category::AppInformation => crate::workspace::DetailTab::Information,
      appforge_compliance::Category::AppReview => crate::workspace::DetailTab::Review,
      appforge_compliance::Category::AppPrivacy => crate::workspace::DetailTab::Privacy,
    };
    let button = Button::new(SharedString::from(format!("go-{}-{}", finding.id, finding.title)))
      .label(format!("Open {}", finding.category.label()))
      .small()
      .outline()
      .on_click(cx.listener(move |this, _, _, cx| {
        this.tab = tab;
        cx.notify();
      }))
      .into_any_element();
    finding_card(finding, Some(button), cx)
  }

  pub(crate) fn render_analysis(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
    let theme = cx.theme().clone();
    let Some(report) = self.report.clone() else { return v_flex().into_any_element() };
    let (verdict, color) = match report.verdict() {
      Verdict::NotReady => ("Not ready for review", theme.danger),
      Verdict::NeedsAttention => ("Almost ready", theme.warning),
      Verdict::LooksReady => ("Looks ready", theme.success),
    };
    let provider = self.settings.ai.provider;
    let has_key = self.has_key.get(&provider).copied().unwrap_or(false);
    let summary = h_flex()
      .gap_4()
      .items_center()
      .p_4()
      .rounded_md()
      .border_1()
      .border_color(theme.border)
      .child(div().text_color(color).text_size(rems(2.5)).font_weight(FontWeight::BOLD).child(report.score().to_string()))
      .child(v_flex().gap_1().child(div().font_weight(FontWeight::SEMIBOLD).text_color(color).child(verdict)).child(
        div().text_sm().text_color(theme.muted_foreground).child(format!(
          "{} critical · {} · {}",
          report.count(Severity::Critical),
          count(report.count(Severity::Warning), "warning"),
          count(report.count(Severity::Info), "tip")
        )),
      ));
    let findings = v_flex().gap_2().children(report.findings.iter().map(|finding| self.finding_with_action(finding, cx)));
    let findings = if report.findings.is_empty() {
      v_flex().child(div().text_sm().text_color(theme.muted_foreground).child("No issues found in the saved data."))
    } else {
      findings
    };
    let ai_body = match &self.ai {
      AiState::Idle => v_flex().into_any_element(),
      AiState::Running => h_flex()
        .gap_2()
        .items_center()
        .text_sm()
        .text_color(theme.muted_foreground)
        .child(Spinner::new())
        .child(format!("{} is reading your app and the rules…", provider.label()))
        .into_any_element(),
      AiState::Failed(message) => Alert::error("ai-error", message.clone()).into_any_element(),
      AiState::Done(review) => {
        let mut body = v_flex().gap_2();
        if !review.summary.is_empty() {
          body = body.child(div().text_sm().child(review.summary.clone()));
        }
        if review.findings.is_empty() {
          body = body.child(div().text_sm().text_color(theme.muted_foreground).child("The AI found nothing more to add."));
        }
        body.children(review.findings.iter().map(|finding| self.finding_with_action(finding, cx))).into_any_element()
      }
    };
    let ai_hint: SharedString = if has_key {
      format!("Sends the listing text, the review notes and your privacy answers to {} together with the rules below. Demo passwords and contact details are never sent.", provider.label()).into()
    } else {
      format!("Add a {} API key in Settings to run the AI analysis.", provider.label()).into()
    };
    let playbook_status: SharedString = match (self.playbook.fetched_at, self.playbook.is_empty()) {
      (_, true) => "Not downloaded yet".into(),
      (Some(fetched), false) => format!("Updated {}", ago(fetched)).into(),
      (None, false) => "Downloaded".into(),
    };
    v_flex()
      .gap_4()
      .child(page_title("Review readiness", "Based on the data saved in App Store Connect and your App Privacy answers. Apple's reviewers also test the app itself, so no check can promise approval.", cx))
      .child(summary)
      .child(findings)
      .child(div().pt_4().text_lg().font_weight(FontWeight::SEMIBOLD).child("AI analysis"))
      .child(div().text_sm().text_color(theme.muted_foreground).child(ai_hint))
      .child(
        h_flex().gap_2().child(
          Button::new("run-ai")
            .icon(Icon::new(IconName::Bot))
            .label(format!("Analyze with {}", provider.label()))
            .primary()
            .loading(matches!(self.ai, AiState::Running))
            .disabled(!has_key)
            .on_click(cx.listener(|this, _, window, cx| this.run_ai(window, cx))),
        ),
      )
      .child(ai_body)
      .child(
        h_flex()
          .pt_4()
          .gap_2()
          .items_center()
          .text_sm()
          .text_color(theme.muted_foreground)
          .child(format!("Rules: App Store Compliance Playbook · {playbook_status}"))
          .child(Button::new("open-playbook").label("View").ghost().small().on_click(|_, _, cx| cx.open_url(PLAYBOOK_REPOSITORY)))
          .child(
            Button::new("update-playbook")
              .label("Update rules")
              .ghost()
              .small()
              .loading(self.playbook_busy)
              .on_click(cx.listener(|this, _, window, cx| this.refresh_playbook(true, window, cx))),
          ),
      )
      .into_any_element()
  }
}
