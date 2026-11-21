//! Small pieces shared by several screens.

use std::path::PathBuf;

use crate::{
  i18n::tr,
  theme::{avatar_color, colors, Colors, FONT_SIZE, FONT_SIZE_SM},
};
use appforge_compliance::Severity;
use appforge_storage::Language;
use gpui_kit::{
  component::{
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputState, Textarea, TextareaState},
    label::Label,
    separator::Separator,
    tooltip::Tooltip,
    v_flex, ActiveTheme, Disableable as _, Icon, IconName, Sizable as _,
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

pub(crate) struct FieldLabel {
  text: SharedString,
  required: bool,
}

impl From<SharedString> for FieldLabel {
  fn from(text: SharedString) -> Self {
    Self { text, required: false }
  }
}

pub(crate) fn required(text: SharedString) -> FieldLabel {
  FieldLabel { text, required: true }
}

pub(crate) fn required_mark(cx: &App) -> impl IntoElement {
  div().text_color(colors(cx).danger).child("*")
}

fn labelled(id: EntityId, label: FieldLabel, counter: Option<impl IntoElement>, input: impl IntoElement, cx: &App) -> impl IntoElement {
  let title = h_flex().gap_1().child(Label::new(label.text).id(("field-label", id))).children(label.required.then(|| required_mark(cx)));
  v_flex().gap_1().child(h_flex().justify_between().child(title).children(counter)).child(input)
}

/// A one-line field. `limit` shows a live character counter.
pub fn text_field(label: impl Into<FieldLabel>, input: &Entity<InputState>, limit: Option<usize>, cx: &App) -> impl IntoElement {
  let length = input.read(cx).value().chars().count();
  labelled(input.entity_id(), label.into(), counter(length, limit, cx), Input::new(input), cx)
}

/// The web address to open for what was typed in a URL field: `https://` is assumed when there is no scheme, and only
/// `http` and `https` addresses open. `None` for an empty or unusable value.
fn openable_url(value: &str) -> Option<String> {
  let value = value.trim();
  if value.is_empty() || value.contains(char::is_whitespace) {
    return None;
  }
  let lower = value.to_lowercase();
  if lower.starts_with("http://") || lower.starts_with("https://") {
    Some(value.to_owned())
  } else if value.contains("://") || !value.contains('.') {
    None
  } else {
    Some(format!("https://{value}"))
  }
}

/// A one-line field for a web address, with a button inside it that opens the address in the browser.
pub fn url_field(id: &'static str, label: impl Into<FieldLabel>, input: &Entity<InputState>, language: Language, cx: &App) -> impl IntoElement {
  let value = input.read(cx).value().to_string();
  let url = openable_url(&value);
  let length = value.chars().count();
  let open =
    Button::new(id).icon(Icon::new(IconName::ExternalLink)).ghost().xsmall().disabled(url.is_none()).tooltip(tr(language, "openInBrowser")).on_click(
      move |_, _, cx| {
        if let Some(url) = &url {
          cx.open_url(url);
        }
      },
    );
  labelled(input.entity_id(), label.into(), counter(length, None, cx), Input::new(input).suffix(open), cx)
}

/// A multi-line field.
pub fn area_field(label: impl Into<FieldLabel>, input: &Entity<TextareaState>, limit: Option<usize>, cx: &App) -> impl IntoElement {
  let length = input.read(cx).value().chars().count();
  labelled(input.entity_id(), label.into(), counter(length, limit, cx), Textarea::new(input), cx)
}

/// A listing input with its generation action inside the field.
pub(crate) fn text_field_with_action(
  label: impl Into<FieldLabel>,
  input: &Entity<InputState>,
  limit: usize,
  action: impl IntoElement,
  cx: &App,
) -> impl IntoElement {
  let length = input.read(cx).value().chars().count();
  labelled(input.entity_id(), label.into(), counter(length, Some(limit), cx), Input::new(input).suffix(action), cx)
}

/// A textarea with its action beside the character counter above the field, so nothing sits over the text and the
/// scrollbar stays at the far right edge of the field.
pub(crate) fn area_field_with_action(
  label: impl Into<FieldLabel>,
  input: &Entity<TextareaState>,
  limit: usize,
  action: impl IntoElement,
  cx: &App,
) -> impl IntoElement {
  let length = input.read(cx).value().chars().count();
  let header = h_flex().gap_2().items_center().child(action).children(counter(length, Some(limit), cx));
  labelled(input.entity_id(), label.into(), Some(header), Textarea::new(input), cx)
}

/// Centers a form in a readable column.
pub fn centered(child: impl IntoElement) -> Div {
  centered_with_width(child, px(CONTENT_MAX_WIDTH))
}

pub(crate) fn centered_with_width(child: impl IntoElement, width: impl Into<Length>) -> Div {
  h_flex().w_full().justify_center().child(div().w_full().min_w_0().max_w(width.into()).px_6().child(child))
}

/// Pages get a second column only when both columns have room beside the navigation.
pub(crate) fn has_room_for_columns(window: &Window) -> bool {
  window.viewport_size().width >= px(ACTIVITY_BAR_WIDTH + SIDEBAR_WIDTH) + window.rem_size() * 65.
}

pub(crate) fn page_columns(wide: bool, primary: impl IntoElement, secondary: impl IntoElement) -> Div {
  page_columns_with_primary(wide, 0.56, primary, secondary)
}

pub(crate) fn page_columns_with_primary(wide: bool, primary_share: f32, primary: impl IntoElement, secondary: impl IntoElement) -> Div {
  if wide {
    h_flex()
      .w_full()
      .gap_6()
      .items_start()
      .child(div().flex_basis(relative(primary_share)).min_w_0().child(primary))
      .child(div().flex_1().min_w_0().child(secondary))
  } else {
    v_flex().w_full().gap_6().child(primary).child(secondary)
  }
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

/// A section of a long page as one calm card: a title in the interface font, then the body. Text sizes and colors come from the theme.
pub fn page_section(title: impl Into<SharedString>, body: impl IntoElement, c: &Colors) -> Div {
  v_flex()
    .w_full()
    .gap_4()
    .p_5()
    .rounded_lg()
    .border_1()
    .border_color(c.border)
    .child(div().text_size(FONT_SIZE).font_weight(FontWeight::SEMIBOLD).child(title.into()))
    .child(div().w_full().child(body))
}

/// A plain text link that opens a page in the browser, as App Store Connect shows its links.
pub fn text_link(id: &'static str, label: impl Into<SharedString>, url: impl Into<String>, c: &Colors) -> Stateful<Div> {
  let url = url.into();
  div().id(id).cursor_pointer().text_color(c.accent).child(label.into()).on_click(move |_, _, cx| cx.open_url(&url))
}

/// A bordered group: a title with an optional hint on the left and an optional action on the right, then the body.
pub fn card(title: impl Into<SharedString>, hint: Option<SharedString>, action: Option<AnyElement>, body: impl IntoElement, c: &Colors) -> Div {
  v_flex()
    .w_full()
    .gap_2()
    .p_4()
    .rounded_lg()
    .border_1()
    .border_color(c.border)
    .child(
      h_flex()
        .w_full()
        .gap_3()
        .flex_wrap()
        .items_center()
        .child(
          v_flex()
            .flex_1()
            .flex_basis(rems(10.))
            .min_w_0()
            .child(div().font_weight(FontWeight::SEMIBOLD).child(title.into()))
            .children(hint.map(|hint| div().text_size(FONT_SIZE_SM).text_color(c.text_secondary).child(hint))),
        )
        .children(action),
    )
    .child(body)
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
    .gap_3()
    .flex_wrap()
    .py_3()
    .items_center()
    .justify_between()
    .border_b_1()
    .border_color(c.border)
    .child(
      v_flex()
        .flex_1()
        .flex_basis(rems(12.))
        .min_w_0()
        .gap_0p5()
        .child(div().font_weight(FontWeight::MEDIUM).child(title.into()))
        .children(description.map(|text| div().text_size(FONT_SIZE_SM).text_color(c.text_secondary).child(text))),
    )
    .child(div().max_w_full().child(control))
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
        .when(ring, |avatar| avatar.border_2().border_color(if selected { accent } else { gpui_kit::transparent_black() }))
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
pub(crate) fn state_label(state: &str) -> String {
  let lower = state.replace('_', " ").to_lowercase();
  let mut characters = lower.chars();
  characters.next().map(|first| first.to_uppercase().collect::<String>() + characters.as_str()).unwrap_or_default()
}

/// Where the version stands at Apple: a dot in the color of the state, then its name.
/// The color of where a version stands at Apple.
pub fn state_color(state: &str, c: &Colors) -> Hsla {
  match state {
    "READY_FOR_SALE" | "READY_FOR_DISTRIBUTION" | "ACCEPTED" | "PENDING_DEVELOPER_RELEASE" => c.success,
    "REJECTED" | "METADATA_REJECTED" | "DEVELOPER_REJECTED" | "INVALID_BINARY" => c.danger,
    "PREPARE_FOR_SUBMISSION" => c.warning,
    _ => c.info,
  }
}

pub fn state_badge(state: &str, c: &Colors) -> Div {
  let color = state_color(state, c);
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
