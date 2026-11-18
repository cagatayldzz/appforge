use crate::{
  i18n::{tr, trf},
  widgets::{finding_card, page_title},
  workspace::{AiState, Workspace},
};
use appforge_compliance::{Finding, Severity, Verdict, PLAYBOOK_REPOSITORY};
use appforge_storage::Language;
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
fn ago(l: Language, fetched_at: u64) -> SharedString {
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

impl Workspace {
  fn finding_with_action(&self, finding: &Finding, cx: &mut Context<Self>) -> impl IntoElement {
    let l = self.settings.language;
    let tab = match finding.category {
      appforge_compliance::Category::AppInformation => crate::workspace::DetailTab::Information,
      appforge_compliance::Category::AppReview => crate::workspace::DetailTab::Review,
      appforge_compliance::Category::AppPrivacy => crate::workspace::DetailTab::Privacy,
    };
    let button = Button::new(SharedString::from(format!("go-{}-{}", finding.id, finding.title)))
      .label(trf(l, "Open {}", &[&tr(l, finding.category.label())]))
      .small()
      .outline()
      .on_click(cx.listener(move |this, _, _, cx| {
        this.tab = tab;
        cx.notify();
      }))
      .into_any_element();
    finding_card(finding, Some(button), l, cx)
  }

  pub(crate) fn render_analysis(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
    let theme = cx.theme().clone();
    let l = self.settings.language;
    let Some(report) = self.report.clone() else { return v_flex().into_any_element() };
    let (verdict, color) = match report.verdict() {
      Verdict::NotReady => (tr(l, "Not ready for review"), theme.danger),
      Verdict::NeedsAttention => (tr(l, "Almost ready"), theme.warning),
      Verdict::LooksReady => (tr(l, "Looks ready"), theme.success),
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
        div().text_sm().text_color(theme.muted_foreground).child(trf(
          l,
          "{} critical · {} · {}",
          &[
            &report.count(Severity::Critical).to_string(),
            &count(l, report.count(Severity::Warning), "{} warning", "{} warnings"),
            &count(l, report.count(Severity::Info), "{} tip", "{} tips"),
          ],
        )),
      ));
    let findings = v_flex().gap_2().children(report.findings.iter().map(|finding| self.finding_with_action(finding, cx)));
    let findings = if report.findings.is_empty() {
      v_flex().child(div().text_sm().text_color(theme.muted_foreground).child(tr(l, "No issues found in the saved data.")))
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
        .child(trf(l, "{} is reading your app and the rules…", &[provider.label()]))
        .into_any_element(),
      AiState::Failed(message) => Alert::error("ai-error", message.clone()).into_any_element(),
      AiState::Done(review) => {
        let mut body = v_flex().gap_2();
        if !review.summary.is_empty() {
          body = body.child(div().text_sm().child(review.summary.clone()));
        }
        if review.findings.is_empty() {
          body = body.child(div().text_sm().text_color(theme.muted_foreground).child(tr(l, "The AI found nothing more to add.")));
        }
        body.children(review.findings.iter().map(|finding| self.finding_with_action(finding, cx))).into_any_element()
      }
    };
    let ai_hint: SharedString = if has_key {
      trf(l, "Sends the listing text, the review notes and your privacy answers to {} together with the rules below. Demo passwords and contact details are never sent.", &[provider.label()])
    } else {
      trf(l, "Add a {} API key in Settings to run the AI analysis.", &[provider.label()])
    };
    let playbook_status: SharedString = match (self.playbook.fetched_at, self.playbook.is_empty()) {
      (_, true) => tr(l, "Not downloaded yet"),
      (Some(fetched), false) => trf(l, "Updated {}", &[&ago(l, fetched)]),
      (None, false) => tr(l, "Downloaded"),
    };
    v_flex()
      .gap_4()
      .child(page_title(tr(l, "Review readiness"), tr(l, "Based on the data saved in App Store Connect and your App Privacy answers. Apple's reviewers also test the app itself, so no check can promise approval."), cx))
      .child(summary)
      .child(findings)
      .child(div().pt_4().text_lg().font_weight(FontWeight::SEMIBOLD).child(tr(l, "AI analysis")))
      .child(div().text_sm().text_color(theme.muted_foreground).child(ai_hint))
      .child(
        h_flex().gap_2().child(
          Button::new("run-ai")
            .icon(Icon::new(IconName::Bot))
            .label(trf(l, "Analyze with {}", &[provider.label()]))
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
          .child(trf(l, "Rules: App Store Compliance Playbook · {}", &[&playbook_status]))
          .child(Button::new("open-playbook").label(tr(l, "View")).ghost().small().on_click(|_, _, cx| cx.open_url(PLAYBOOK_REPOSITORY)))
          .child(
            Button::new("update-playbook")
              .label(tr(l, "Update rules"))
              .ghost()
              .small()
              .loading(self.playbook_busy)
              .on_click(cx.listener(|this, _, window, cx| this.refresh_playbook(true, window, cx))),
          ),
      )
      .into_any_element()
  }
}
