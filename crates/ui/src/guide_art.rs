//! Cards and small drawings for the welcome and sign-in screens: they show what to click and what to copy.
//! Everything is built from GPUI Kit components. The words inside the drawings are Apple's own interface text,
//! so they are not translated.

use crate::i18n::tr;
use appforge_storage::Language;
use gpui_kit::{
  component::{
    attachment::{Attachment, AttachmentContent, AttachmentDescription, AttachmentMedia, AttachmentTitle},
    badge::Badge,
    checkbox::Checkbox,
    clipboard::Clipboard,
    group_box::{GroupBox, GroupBoxVariants as _},
    h_flex,
    input::{Input, InputState},
    label::Label,
    tag::Tag,
    v_flex, ActiveTheme, Icon, IconName, Sizable as _,
  },
  *,
};
use std::time::Duration;

/// How long one pulse of a highlight takes.
const PULSE_MS: u64 = 1600;
/// How long an element takes to fade in.
const REVEAL_MS: u64 = 450;
/// How far an element rises while it fades in.
const RISE_DISTANCE: f32 = 14.;
/// Cards that appear together start this much apart.
const STAGGER_MS: u64 = 120;
/// Width of the column of drawings, so the numbers, the texts and the drawings line up.
pub const DRAWING_WIDTH: f32 = 400.;

/// Text that Apple shows in its own interface.
fn apple(text: &'static str) -> SharedString {
  SharedString::new_static(text)
}

/// Fades and lifts `element` in when it first appears.
pub fn reveal(id: impl Into<ElementId>, element: Div) -> impl IntoElement {
  reveal_after(id, 0, element)
}

/// Like `reveal`, starting `delay_ms` later, so a list can appear one item after another.
pub fn reveal_after(id: impl Into<ElementId>, delay_ms: u64, element: Div) -> impl IntoElement {
  let total = (delay_ms + REVEAL_MS) as f32;
  element.relative().with_animation(id, Animation::new(Duration::from_millis(delay_ms + REVEAL_MS)), move |element, delta| {
    let progress = ease_out_quint()(((delta * total - delay_ms as f32) / REVEAL_MS as f32).clamp(0., 1.));
    element.opacity(progress).top(px(RISE_DISTANCE * (1. - progress)))
  })
}

/// A breathing highlight: `element` fades between `low` and `high`.
fn pulse(id: impl Into<ElementId>, low: f32, high: f32, element: Div) -> impl IntoElement {
  element.with_animation(id, Animation::new(Duration::from_millis(PULSE_MS)).repeat().with_easing(pulsating_between(low, high)), |element, value| {
    element.opacity(value)
  })
}

/// A thing to click: it carries a dot and breathes.
fn marked(id: &'static str, child: impl IntoElement, cx: &App) -> impl IntoElement {
  pulse(id, 0.55, 1., div().child(Badge::new().dot().color(cx.theme().info).child(child)))
}

/// How many things there are to do on Apple's site in the first step.
pub const KEY_STEP_COUNT: usize = 5;
/// The least width the text of a row keeps before the drawing wraps below it.
const ROW_TEXT_MIN_WIDTH: f32 = 260.;

/// One thing to do: a checkbox, a title and a line of help on the left, a drawing of the place to click on the right.
/// A ticked row fades, so what is left to do stands out. The drawing wraps below the text in a narrow window.
pub fn step_row(
  index: usize,
  done: Checkbox,
  ticked: bool,
  title: SharedString,
  text: SharedString,
  caption: SharedString,
  drawing: Div,
  cx: &App,
) -> impl IntoElement {
  let muted = cx.theme().muted_foreground;
  let words = v_flex()
    .flex_1()
    .min_w(px(ROW_TEXT_MIN_WIDTH))
    .gap_1()
    .opacity(if ticked { 0.55 } else { 1. })
    .child(div().font_weight(FontWeight::SEMIBOLD).child(title))
    .child(div().text_sm().text_color(muted).child(text));
  let row = h_flex()
    .w_full()
    .flex_wrap()
    .gap_x_6()
    .gap_y_3()
    .items_start()
    .py_4()
    .child(h_flex().flex_1().min_w(px(ROW_TEXT_MIN_WIDTH)).gap_3().items_start().child(div().pt_0p5().child(done)).child(words))
    .child(v_flex().w(px(DRAWING_WIDTH)).flex_shrink_0().gap_1p5().child(div().text_xs().text_color(muted).child(caption)).child(drawing));
  reveal_after(SharedString::from(format!("step-row-{index}")), index as u64 * STAGGER_MS, row)
}

/// A small copy of one of Apple's windows: its title, what it shows and its buttons.
fn apple_window(title: &'static str, content: impl IntoElement, buttons: impl IntoElement) -> GroupBox {
  GroupBox::new()
    .outline()
    .title(Label::new(apple(title)))
    .child(v_flex().w_full().gap_2().child(content).child(h_flex().justify_end().gap_2().child(buttons)))
}

/// Where to click to start a key: the Generate API Key button, or the + next to Active when there are keys already.
pub fn generate_key_drawing(language: Language, cx: &App) -> Div {
  h_flex()
    .gap_2()
    .items_center()
    .child(marked("mark-generate-key", Tag::primary().child(apple("Generate API Key")), cx))
    .child(div().text_xs().text_color(cx.theme().muted_foreground).child(tr(language, "or")))
    .child(Tag::primary().child(Icon::new(IconName::Plus)))
}

/// Apple's Generate API Key window. The name to type there can be copied from the field.
pub fn name_window_drawing(language: Language, name: &Entity<InputState>, cx: &App) -> Div {
  div().child(apple_window(
    "Generate API Key",
    v_flex()
      .gap_1p5()
      .child(Label::new(apple("Name")))
      .child(Input::new(name).small().suffix(Clipboard::new("copy-key-name").value("AppForge").tooltip(tr(language, "copyTheName"))))
      .child(Label::new(apple("Access")))
      .child(
        h_flex()
          .gap_2()
          .items_center()
          .child(marked("mark-roles", Tag::secondary().outline().child(apple("Select Roles")), cx))
          .child(Icon::new(IconName::ChevronRight).small())
          .child(Tag::secondary().child(apple("App Manager"))),
      ),
    h_flex().gap_2().child(Tag::secondary().outline().child(apple("Cancel"))).child(marked(
      "mark-generate",
      Tag::primary().child(apple("Generate")),
      cx,
    )),
  ))
}

/// The new key in Apple's list, with its Download link.
pub fn download_drawing(cx: &App) -> Div {
  h_flex()
    .w_full()
    .gap_3()
    .items_center()
    .justify_between()
    .child(Label::new(apple("AppForge")))
    .child(Tag::secondary().child(apple("ABC123DEFG")))
    .child(marked("mark-download", Tag::secondary().child(apple("Download")), cx))
}

/// Apple's warning that a key can be downloaded only once.
pub fn confirm_drawing(cx: &App) -> Div {
  div().child(apple_window(
    "Download API Key",
    div().text_xs().text_color(cx.theme().muted_foreground).child(apple("API keys can be downloaded only once.")),
    h_flex().gap_2().child(Tag::secondary().outline().child(apple("Cancel"))).child(marked(
      "mark-confirm",
      Tag::primary().child(apple("Download")),
      cx,
    )),
  ))
}

/// Step 2: how Apple shows the Issuer ID: its label, the value and a Copy link.
pub fn issuer_drawing(cx: &App) -> Div {
  v_flex().gap_1().child(Label::new(apple("Issuer ID"))).child(
    h_flex()
      .gap_2()
      .items_center()
      .child(marked("mark-issuer", div().text_sm().child(apple("a1b2c3d4-0000-0000-0000-e5f6a7b8c9d0")), cx))
      .child(Tag::secondary().child(apple("Copy"))),
  )
}

/// Step 2: where the Key ID is, in the table of keys: the columns Apple shows, without the empty Last Used.
pub fn key_id_drawing(cx: &App) -> Div {
  let muted = cx.theme().muted_foreground;
  let header = |text: &'static str| div().flex_1().min_w_0().whitespace_nowrap().text_xs().text_color(muted).child(apple(text));
  let cell = |text: &'static str| div().flex_1().min_w_0().text_xs().child(apple(text));
  v_flex().gap_1().child(h_flex().gap_2().child(header("NAME")).child(header("GENERATED BY")).child(header("KEY ID")).child(header("ACCESS"))).child(
    h_flex()
      .gap_2()
      .items_center()
      .child(cell("AppForge"))
      .child(cell("Your name"))
      .child(div().flex_1().min_w_0().child(marked("mark-key", Tag::secondary().small().child(apple("ABC123DEFG")), cx)))
      .child(cell("App Manager")),
  )
}

/// Step 3: the downloaded file. `chosen` is the name of the file once it is picked.
pub fn key_file_drawing(chosen: Option<&str>, language: Language) -> Div {
  let file_name: SharedString = chosen.map(|name| SharedString::from(name.to_owned())).unwrap_or_else(|| apple("AuthKey_ABC123DEFG.p8"));
  let file = Attachment::new()
    .id(SharedString::from(format!("key-file-{file_name}")))
    .media(AttachmentMedia::new().child(Icon::new(IconName::FileText)))
    .content(AttachmentContent::new().title(AttachmentTitle::new(file_name.clone())).description(AttachmentDescription::new("p8")));
  let mut row = h_flex().gap_3().items_center().child(file);
  if chosen.is_some() {
    row = row.child(reveal(SharedString::from(format!("file-ok-{file_name}")), div().child(Tag::success().child(tr(language, "ready")))));
  }
  row
}
