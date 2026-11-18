//! Small pieces shared by several screens.

use appforge_compliance::{Finding, Severity};
use gpui_kit::{
  component::{
    h_flex,
    input::{Input, InputState, Textarea, TextareaState},
    label::Label,
    tag::Tag,
    v_flex, ActiveTheme, Sizable as _,
  },
  *,
};

/// Width of the app list on the left.
pub const SIDEBAR_WIDTH: f32 = 240.;
/// The widest a form grows; longer lines are hard to read.
pub const CONTENT_MAX_WIDTH: f32 = 760.;

fn counter(length: usize, limit: Option<usize>, cx: &App) -> Option<impl IntoElement> {
  let limit = limit?;
  let theme = cx.theme();
  let color = if length > limit { theme.danger } else { theme.muted_foreground };
  Some(div().text_xs().text_color(color).child(format!("{length}/{limit}")))
}

fn labelled(label: &'static str, counter: Option<impl IntoElement>, input: impl IntoElement) -> impl IntoElement {
  v_flex().gap_1().child(h_flex().justify_between().child(Label::new(label)).children(counter)).child(input)
}

/// A one-line field. `limit` shows a live character counter.
pub fn text_field(label: &'static str, input: &Entity<InputState>, limit: Option<usize>, cx: &App) -> impl IntoElement {
  let length = input.read(cx).value().chars().count();
  labelled(label, counter(length, limit, cx), Input::new(input))
}

/// A multi-line field.
pub fn area_field(label: &'static str, input: &Entity<TextareaState>, limit: Option<usize>, cx: &App) -> impl IntoElement {
  let length = input.read(cx).value().chars().count();
  labelled(label, counter(length, limit, cx), Textarea::new(input))
}

pub fn page_title(title: impl Into<SharedString>, subtitle: impl Into<SharedString>, cx: &App) -> impl IntoElement {
  v_flex()
    .gap_1()
    .child(div().text_2xl().font_weight(FontWeight::BOLD).child(title.into()))
    .child(div().text_sm().text_color(cx.theme().muted_foreground).child(subtitle.into()))
}

pub fn severity_tag(severity: Severity) -> Tag {
  let tag = match severity {
    Severity::Critical => Tag::danger(),
    Severity::Warning => Tag::warning(),
    Severity::Info => Tag::info(),
  };
  tag.small().child(severity.label())
}

/// One finding: severity, title, guideline, what is wrong and how to fix it. `action` is an optional button, for example "Fix".
pub fn finding_card(finding: &Finding, action: Option<AnyElement>, cx: &App) -> impl IntoElement {
  let theme = cx.theme();
  let mut heading = h_flex()
    .flex_1()
    .min_w_0()
    .flex_wrap()
    .gap_x_2()
    .gap_y_1()
    .items_center()
    .child(severity_tag(finding.severity))
    .child(div().font_weight(FontWeight::SEMIBOLD).child(finding.title.clone()));
  if !finding.guideline.is_empty() {
    heading = heading.child(div().text_xs().text_color(theme.muted_foreground).child(format!("Guideline {}", finding.guideline)));
  }
  v_flex()
    .gap_1()
    .p_3()
    .rounded_md()
    .border_1()
    .border_color(theme.border)
    .child(h_flex().justify_between().items_start().gap_2().child(heading).children(action.map(|action| div().flex_shrink_0().child(action))))
    .child(div().text_sm().text_color(theme.muted_foreground).child(finding.detail.clone()))
    .child(div().text_sm().child(format!("Fix: {}", finding.fix)))
}
