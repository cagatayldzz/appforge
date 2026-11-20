//! The progress notification of an AI analysis: the steps with their state, and what is being sent to the provider.

use super::*;
use crate::theme::{colors, FONT_SIZE_SM};
use gpui_kit::component::{h_flex, spinner::Spinner, v_flex, Icon, IconName, Sizable as _};

/// The notification is replaced, not stacked, each time the analysis moves on.
pub(super) struct AiAnalysisNotice;

/// The steps of an analysis, in order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Stage {
  CheckingKey,
  ReadingSource,
  LoadingRules,
  Preparing,
  Waiting,
  Retrying,
  ReadingResponse,
}

impl Stage {
  /// The position among the visible steps; a retry belongs to the waiting step and the rules are part of preparing.
  fn step(self) -> usize {
    match self {
      Self::CheckingKey => 0,
      Self::ReadingSource => 1,
      Self::LoadingRules | Self::Preparing => 2,
      Self::Waiting | Self::Retrying => 3,
      Self::ReadingResponse => 4,
    }
  }

  fn label(self, project: bool) -> &'static str {
    match self.step() {
      0 => "projectAnalysisStepKey",
      1 if project => "projectAnalysisStepSource",
      1 => "",
      2 => "projectAnalysisStepData",
      3 => "projectAnalysisStepRequest",
      _ => "projectAnalysisStepResponse",
    }
  }
}

/// What the current request contains, shown under the steps.
#[derive(Clone)]
pub(super) struct Sent {
  pub provider: SharedString,
  pub model: SharedString,
  pub project: bool,
  pub findings: usize,
}

/// Shows (or replaces) the progress notification. `message` is the sentence of the current step.
pub(super) fn show(stage: Stage, message: SharedString, language: Language, sent: &Sent, window: &mut Window, cx: &mut App) {
  let sent = sent.clone();
  let notice = Notification::info(message.clone())
    .id::<AiAnalysisNotice>()
    .title(tr(language, "projectAnalysisTitle"))
    .autohide(false)
    .content(move |_, _, cx| render(stage, language, &sent, cx));
  window.push_notification(notice, cx);
}

fn render(stage: Stage, language: Language, sent: &Sent, cx: &App) -> AnyElement {
  let c = colors(cx);
  let current = stage.step();
  let steps = (0..5).filter(|step| *step != 1 || sent.project).map(|step| {
    let label = [Stage::CheckingKey, Stage::ReadingSource, Stage::Preparing, Stage::Waiting, Stage::ReadingResponse][step].label(sent.project);
    let marker = if step < current {
      Icon::new(IconName::Check).small().text_color(c.success).into_any_element()
    } else if step == current {
      Spinner::new().small().into_any_element()
    } else {
      div().size_2().rounded_full().border_1().border_color(c.border).into_any_element()
    };
    let text = if step == current { c.text } else { c.text_secondary };
    h_flex().gap_2().items_center().text_size(FONT_SIZE_SM).text_color(text).child(div().w_4().flex_none().child(marker)).child(tr(language, label))
  });
  let mut sending = vec![
    trf(language, "projectAnalysisSendsProvider", &[&sent.provider, &sent.model]),
    tr(language, "projectAnalysisSendsListing"),
    trf(language, "projectAnalysisSendsFindings", &[&sent.findings.to_string()]),
  ];
  if sent.project {
    sending.push(tr(language, "projectAnalysisSendsProject"));
  }
  v_flex()
    .gap_2()
    .pt_1()
    .child(v_flex().gap_1().children(steps))
    .child(
      v_flex()
        .gap_0p5()
        .pt_2()
        .border_t_1()
        .border_color(c.border)
        .text_size(FONT_SIZE_SM)
        .text_color(c.text_secondary)
        .child(div().font_weight(FontWeight::SEMIBOLD).child(tr(language, "projectAnalysisSendsHeading")))
        .children(sending),
    )
    .into_any_element()
}
