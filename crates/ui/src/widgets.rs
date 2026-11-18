//! Small pieces shared by several screens.

use crate::i18n::{tr, trf};
use appforge_compliance::{Finding, Severity};
use appforge_storage::Language;
use gpui_kit::{
  component::{
    group_box::{GroupBox, GroupBoxVariants as _},
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
pub const CONTENT_MAX_WIDTH: f32 = 880.;

fn counter(length: usize, limit: Option<usize>, cx: &App) -> Option<impl IntoElement> {
  let limit = limit?;
  let theme = cx.theme();
  let color = if length > limit { theme.danger } else { theme.muted_foreground };
  Some(div().text_xs().text_color(color).child(format!("{length}/{limit}")))
}

fn labelled(label: SharedString, counter: Option<impl IntoElement>, input: impl IntoElement) -> impl IntoElement {
  v_flex().gap_1().child(h_flex().justify_between().child(Label::new(label)).children(counter)).child(input)
}

/// A one-line field. `limit` shows a live character counter.
pub fn text_field(label: impl Into<SharedString>, input: &Entity<InputState>, limit: Option<usize>, cx: &App) -> impl IntoElement {
  let length = input.read(cx).value().chars().count();
  labelled(label.into(), counter(length, limit, cx), Input::new(input))
}

/// A multi-line field.
pub fn area_field(label: impl Into<SharedString>, input: &Entity<TextareaState>, limit: Option<usize>, cx: &App) -> impl IntoElement {
  let length = input.read(cx).value().chars().count();
  labelled(label.into(), counter(length, limit, cx), Textarea::new(input))
}

pub fn page_title(title: impl Into<SharedString>, subtitle: impl Into<SharedString>, cx: &App) -> impl IntoElement {
  v_flex()
    .gap_1()
    .child(div().text_2xl().font_weight(FontWeight::SEMIBOLD).child(title.into()))
    .child(div().text_sm().text_color(cx.theme().muted_foreground).child(subtitle.into()))
}

/// Centers `child` in a column of at most `CONTENT_MAX_WIDTH`, so every page lines up.
pub fn centered(child: impl IntoElement) -> Div {
  h_flex().w_full().justify_center().child(div().w_full().max_w(px(CONTENT_MAX_WIDTH)).px_8().child(child))
}

/// A titled group of fields.
pub fn section(title: impl Into<SharedString>, body: impl IntoElement) -> GroupBox {
  GroupBox::new().outline().title(Label::new(title.into())).child(v_flex().w_full().gap_4().child(body))
}

/// Two fields side by side.
pub fn pair(first: impl IntoElement, second: impl IntoElement) -> Div {
  h_flex().w_full().gap_4().items_start().child(div().flex_1().min_w_0().child(first)).child(div().flex_1().min_w_0().child(second))
}

pub fn severity_tag(severity: Severity, language: Language) -> Tag {
  let tag = match severity {
    Severity::Critical => Tag::danger(),
    Severity::Warning => Tag::warning(),
    Severity::Info => Tag::info(),
  };
  tag.small().child(tr(language, severity.label()))
}

/// One finding: severity, title, guideline, what is wrong and how to fix it. `action` is an optional button, for example "Fix".
pub fn finding_card(finding: &Finding, action: Option<AnyElement>, language: Language, cx: &App) -> impl IntoElement {
  let theme = cx.theme();
  let mut heading = h_flex()
    .flex_1()
    .min_w_0()
    .flex_wrap()
    .gap_x_2()
    .gap_y_1()
    .items_center()
    .child(severity_tag(finding.severity, language))
    .child(div().font_weight(FontWeight::SEMIBOLD).child(finding.title.clone()));
  if !finding.guideline.is_empty() {
    heading = heading.child(div().text_xs().text_color(theme.muted_foreground).child(trf(language, "Guideline {}", &[&finding.guideline])));
  }
  GroupBox::new()
    .outline()
    .title(
      h_flex().w_full().justify_between().items_start().gap_2().child(heading).children(action.map(|action| div().flex_shrink_0().child(action))),
    )
    .child(
      v_flex()
        .w_full()
        .gap_1()
        .child(div().text_sm().text_color(theme.muted_foreground).child(finding.detail.clone()))
        .child(div().text_sm().child(trf(language, "Fix: {}", &[&finding.fix]))),
    )
}
