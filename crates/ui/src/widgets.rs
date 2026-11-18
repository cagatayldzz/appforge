//! Small pieces shared by several screens.

use std::path::PathBuf;

use crate::{
  i18n::{tr, trf},
  theme::{avatar_color, colors, Colors, FONT_SIZE, FONT_SIZE_SM},
};
use appforge_compliance::{Finding, Severity};
use appforge_storage::Language;
use gpui_kit::{
  component::{
    h_flex,
    input::{Input, InputState, Textarea, TextareaState},
    label::Label,
    separator::Separator,
    tooltip::Tooltip,
    v_flex, ActiveTheme, Icon, IconName,
  },
  prelude::FluentBuilder as _,
  *,
};

/// Width of the app list on the left.
pub const SIDEBAR_WIDTH: f32 = 280.;
/// Width of the activity bar, the strip of icon buttons on the far left.
pub const ACTIVITY_BAR_WIDTH: f32 = 56.;
/// The widest a form grows; longer lines are hard to read.
pub const CONTENT_MAX_WIDTH: f32 = 760.;

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

/// Centers `child` in a column of at most `CONTENT_MAX_WIDTH`, so every page lines up.
pub fn centered(child: impl IntoElement) -> Div {
  h_flex().w_full().justify_center().child(div().w_full().max_w(px(CONTENT_MAX_WIDTH)).px_6().child(child))
}

/// A titled group of fields: a heading with a line under it, then the fields. No boxes, so pages stay calm.
pub fn section(title: impl Into<SharedString>, body: impl IntoElement) -> Div {
  v_flex()
    .w_full()
    .gap_3()
    .child(div().text_size(FONT_SIZE).font_weight(FontWeight::SEMIBOLD).child(title.into()))
    .child(Separator::horizontal())
    .child(div().w_full().pt_1().child(body))
}

/// Two fields side by side.
pub fn pair(first: impl IntoElement, second: impl IntoElement) -> Div {
  h_flex().w_full().gap_4().items_start().child(div().flex_1().min_w_0().child(first)).child(div().flex_1().min_w_0().child(second))
}

/// A settings row: the name and what it does on the left, the control on the right, a line under it.
pub fn setting_row(title: impl Into<SharedString>, description: Option<SharedString>, control: impl IntoElement, cx: &App) -> Div {
  let c = colors(cx);
  h_flex()
    .w_full()
    .gap_6()
    .py_3()
    .items_center()
    .justify_between()
    .border_b_1()
    .border_color(c.border)
    .child(
      v_flex()
        .flex_1()
        .min_w_0()
        .gap_0p5()
        .child(div().font_weight(FontWeight::MEDIUM).child(title.into()))
        .children(description.map(|text| div().text_size(FONT_SIZE_SM).text_color(c.text_secondary).child(text))),
    )
    .child(div().flex_shrink_0().child(control))
}

/// A square icon button of the activity bar. The Kit's `Button` fixes its icon at a small size, so this draws the icon itself.
pub fn rail_button(
  id: &'static str,
  icon: IconName,
  tooltip: SharedString,
  selected: bool,
  cx: &App,
  on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
  let c = colors(cx);
  let hover = c.rail_hover;
  div()
    .id(id)
    .size_10()
    .flex()
    .flex_none()
    .items_center()
    .justify_center()
    .rounded_md()
    .cursor_pointer()
    .text_color(if selected { c.text } else { c.rail_text })
    .when(selected, |button| button.bg(hover))
    .hover(move |button| button.bg(hover))
    .child(Icon::new(icon).size_5())
    .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
    .on_click(on_click)
}

/// An app's avatar: its icon when it is known, else its first letter on a tinted square in a color that is always the same
/// for the same app. The chosen app has a ring around its icon (when `ring`) or a filled square for its letter.
pub fn app_avatar(id: &str, name: &str, selected: bool, ring: bool, icon: Option<&PathBuf>, cx: &App) -> Div {
  let color = avatar_color(id);
  let accent = colors(cx).accent;
  let base = div().size_8().flex().flex_none().items_center().justify_center().rounded_lg();
  match icon {
    Some(path) => base
      .border_2()
      .border_color(if selected && ring { accent } else { gpui_kit::transparent_black() })
      .overflow_hidden()
      .child(img(path.clone()).size_full().rounded_md().object_fit(ObjectFit::Cover)),
    None => {
      let letter: String = name.chars().take(1).flat_map(char::to_uppercase).collect();
      base
        .text_size(FONT_SIZE)
        .font_weight(FontWeight::SEMIBOLD)
        .when(selected, |avatar| avatar.bg(color).text_color(white()))
        .when(!selected, |avatar| avatar.bg(color.opacity(0.2)).text_color(color))
        .child(letter)
    }
  }
}

/// One row of a sidebar list: chosen rows are tinted, others light up when hovered.
pub fn list_row(id: impl Into<ElementId>, selected: bool, cx: &App) -> Stateful<Div> {
  let c = colors(cx);
  h_flex()
    .id(id)
    .w_full()
    .px_2()
    .gap_3()
    .items_center()
    .rounded_md()
    .cursor_pointer()
    .text_size(FONT_SIZE)
    .when(selected, |row| row.bg(c.selected))
    .when(!selected, |row| row.hover(move |row| row.bg(c.hover)))
}

/// "PREPARE_FOR_SUBMISSION" as "Prepare for submission".
fn state_label(state: &str) -> String {
  let lower = state.replace('_', " ").to_lowercase();
  let mut characters = lower.chars();
  characters.next().map(|first| first.to_uppercase().collect::<String>() + characters.as_str()).unwrap_or_default()
}

/// Where the version stands at Apple: a dot in the color of the state, then its name.
pub fn state_badge(state: &str, c: &Colors) -> Div {
  let color = match state {
    "READY_FOR_SALE" | "READY_FOR_DISTRIBUTION" | "ACCEPTED" | "PENDING_DEVELOPER_RELEASE" => c.success,
    "REJECTED" | "METADATA_REJECTED" | "DEVELOPER_REJECTED" | "INVALID_BINARY" => c.danger,
    "PREPARE_FOR_SUBMISSION" => c.warning,
    _ => c.info,
  };
  h_flex().gap_1p5().items_center().child(div().size_2().rounded_full().bg(color)).child(div().text_color(c.text).child(state_label(state)))
}

/// How serious a finding is: the name in the color of its severity on a light tint of it. The Kit's tags are hard to read
/// in the dark theme, where their text and fill are nearly the same color.
pub fn severity_badge(severity: Severity, language: Language, cx: &App) -> Div {
  let c = colors(cx);
  let color = match severity {
    Severity::Critical => c.danger,
    Severity::Warning => c.warning,
    Severity::Info => c.info,
  };
  div()
    .px_1p5()
    .py_0p5()
    .rounded_md()
    .bg(color.opacity(0.16))
    .text_color(color)
    .text_size(FONT_SIZE_SM)
    .font_weight(FontWeight::SEMIBOLD)
    .child(tr(language, severity.label()))
}

/// One finding: severity, title, guideline, what is wrong and how to fix it. `action` is an optional button, for example "Fix".
pub fn finding_card(finding: &Finding, action: Option<AnyElement>, language: Language, cx: &App) -> impl IntoElement {
  let c = colors(cx);
  let mut heading = h_flex()
    .flex_1()
    .min_w_0()
    .flex_wrap()
    .gap_x_2()
    .gap_y_1()
    .items_center()
    .child(severity_badge(finding.severity, language, cx))
    .child(div().font_weight(FontWeight::SEMIBOLD).child(finding.title.clone()));
  if !finding.guideline.is_empty() {
    heading = heading.child(div().text_size(FONT_SIZE_SM).text_color(c.text_secondary).child(trf(language, "Guideline {}", &[&finding.guideline])));
  }
  v_flex()
    .w_full()
    .gap_1()
    .py_3()
    .border_b_1()
    .border_color(c.border)
    .child(
      h_flex().w_full().justify_between().items_start().gap_2().child(heading).children(action.map(|action| div().flex_shrink_0().child(action))),
    )
    .child(div().text_color(c.text_secondary).child(finding.detail.clone()))
    .child(div().child(trf(language, "Fix: {}", &[&finding.fix])))
}
