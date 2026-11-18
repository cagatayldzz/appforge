//! Small animated drawings for the welcome and sign-in screens: they show what to click and what to copy.
//! The words inside the drawings are Apple's own interface text, so they are not translated.

use crate::i18n::tr;
use appforge_storage::Language;
use gpui_kit::{
  component::{h_flex, v_flex, ActiveTheme, Icon, IconName, Sizable as _},
  *,
};
use std::time::Duration;

/// How long one pulse of a highlight takes.
const PULSE_MS: u64 = 1600;
/// How long an element takes to fade in.
const REVEAL_MS: u64 = 450;
/// How far an element rises while it fades in.
const RISE_DISTANCE: f32 = 14.;

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

/// A browser window around `body`.
fn browser(address: &'static str, body: impl IntoElement, cx: &App) -> Div {
  let theme = cx.theme();
  v_flex()
    .w_full()
    .rounded_lg()
    .border_1()
    .border_color(theme.border)
    .bg(theme.background)
    .overflow_hidden()
    .child(
      h_flex()
        .gap_1p5()
        .items_center()
        .px_3()
        .py_2()
        .bg(theme.secondary)
        .border_b_1()
        .border_color(theme.border)
        .children((0..3).map(|_| div().size_2().rounded_full().bg(theme.muted_foreground.opacity(0.4))))
        .child(
          div().flex_1().ml_2().px_2().py_0p5().rounded_md().bg(theme.background).text_xs().text_color(theme.muted_foreground).child(apple(address)),
        ),
    )
    .child(body)
}

/// A small numbered circle that ties a drawing to a value.
fn badge(number: usize, cx: &App) -> Div {
  let theme = cx.theme();
  div()
    .flex_shrink_0()
    .size_5()
    .rounded_full()
    .flex()
    .items_center()
    .justify_center()
    .text_xs()
    .font_weight(FontWeight::SEMIBOLD)
    .bg(theme.primary)
    .text_color(theme.primary_foreground)
    .child(number.to_string())
}

/// A value to copy: a chip with a number and a pulsing outline.
fn highlighted(id: &'static str, number: usize, value: &'static str, cx: &App) -> Div {
  let theme = cx.theme();
  let color = theme.primary;
  h_flex().gap_1p5().items_center().child(badge(number, cx)).child(
    div().px_2().py_0p5().rounded_md().border_1().border_color(color).text_xs().font_weight(FontWeight::SEMIBOLD).child(value).with_animation(
      id,
      Animation::new(Duration::from_millis(PULSE_MS)).repeat().with_easing(pulsating_between(0.02, 0.25)),
      move |element, value| element.bg(color.opacity(value)),
    ),
  )
}

fn skeleton(width: Pixels, cx: &App) -> Div {
  div().h_2().w(width).rounded_full().bg(cx.theme().muted_foreground.opacity(0.18))
}

/// Step 1: Apple's Integrations page, with the + button to click and the download link that appears after.
/// Only generic sample values are drawn.
pub fn create_key(language: Language, cx: &App) -> impl IntoElement {
  let theme = cx.theme();
  let blue = theme.info;
  let muted = theme.muted_foreground;
  let text = |value: &'static str| div().text_xs().child(apple(value));
  let link = |value: &'static str| div().text_xs().text_color(blue).child(apple(value));
  let tab = |value: &'static str, active: bool| {
    let label = link(value).pb_0p5();
    if active {
      label.border_b_2().border_color(blue)
    } else {
      label
    }
  };

  let nav = h_flex()
    .gap_2()
    .items_center()
    .px_3()
    .py_1p5()
    .bg(theme.secondary)
    .border_b_1()
    .border_color(theme.border)
    .child(Icon::new(IconName::LayoutDashboard).small())
    .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).child(apple("App Store Connect")))
    .child(div().flex_1())
    .children(["Apps", "Trends", "Reports"].map(|name| div().text_xs().text_color(muted).child(apple(name))));

  let title = h_flex()
    .gap_3()
    .items_end()
    .child(div().text_sm().font_weight(FontWeight::BOLD).child(apple("Users and Access")))
    .child(tab("People", false))
    .child(tab("Integrations", true))
    .child(tab("Xcode Cloud", false));

  let sidebar = v_flex()
    .flex_shrink_0()
    .whitespace_nowrap()
    .gap_1()
    .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).child(apple("Keys")))
    .child(div().px_2().py_0p5().rounded_md().bg(theme.secondary).child(text("App Store Connect API")))
    .child(div().px_2().text_color(muted).child(text("In-App Purchase")))
    .child(div().px_2().text_color(muted).child(text("Shared Secret")));

  let plus = div()
    .relative()
    .size_8()
    .flex()
    .items_center()
    .justify_center()
    .child(pulse("plus-ring", 0.1, 0.9, div().absolute().size_8().rounded_full().border_2().border_color(blue)))
    .child(
      div()
        .size_5()
        .rounded_full()
        .flex()
        .items_center()
        .justify_center()
        .bg(blue)
        .text_color(theme.primary_foreground)
        .child(Icon::new(IconName::Plus).small()),
    );
  let click_here = pulse(
    "click-here",
    0.5,
    1.,
    div().px_2().py_0p5().rounded_full().bg(theme.primary).text_color(theme.primary_foreground).text_xs().child(tr(language, "Click here")),
  );
  let active = h_flex()
    .gap_1p5()
    .items_center()
    .child(div().text_sm().font_weight(FontWeight::BOLD).child(apple("Active (1)")))
    .child(plus)
    .child(badge(2, cx))
    .child(click_here);

  let column = |value: &'static str| div().flex_1().text_color(muted).child(div().text_xs().child(apple(value)));
  let header =
    h_flex().gap_2().px_2().py_1().border_b_1().border_color(theme.border).child(column("NAME")).child(column("KEY ID")).child(div().flex_1());
  let old_row = h_flex()
    .gap_2()
    .px_2()
    .py_1()
    .items_center()
    .child(div().flex_1().child(skeleton(px(48.), cx)))
    .child(div().flex_1().child(skeleton(px(60.), cx)))
    .child(div().flex_1());
  let new_row = h_flex()
    .gap_2()
    .px_2()
    .py_1()
    .items_center()
    .bg(blue.opacity(0.08))
    .child(div().flex_1().font_weight(FontWeight::SEMIBOLD).child(text("AppForge")))
    .child(div().flex_1().child(text("ABC123DEFG")))
    .child(div().flex_1().child(h_flex().gap_1().items_center().child(badge(4, cx)).child(link("Download"))));

  let main = v_flex()
    .flex_1()
    .min_w_0()
    .gap_2()
    .child(div().text_sm().font_weight(FontWeight::BOLD).child(apple("App Store Connect API")))
    .child(h_flex().gap_3().child(tab("Team Keys", true)).child(tab("Individual Keys", false)))
    .child(skeleton(px(140.), cx))
    .child(h_flex().gap_2().items_center().child(div().text_color(muted).child(text("Issuer ID"))).child(text("00000000-…")).child(link("Copy")))
    .child(active)
    .child(v_flex().border_t_1().border_color(theme.border).child(header).child(old_row).child(reveal_after("new-key-row", 700, new_row)));

  let body = v_flex().gap_3().child(title).child(h_flex().gap_3().items_start().child(sidebar).child(main));
  v_flex().w_full().child(browser("appstoreconnect.apple.com/access/integrations/api", v_flex().child(nav).child(div().p_3().child(body)), cx))
}

/// Step 2: where the Issuer ID and the Key ID are on the keys page.
pub fn copy_ids(cx: &App) -> impl IntoElement {
  let theme = cx.theme();
  let muted = theme.muted_foreground;
  let header = |text: &'static str| div().flex_1().text_xs().text_color(muted).child(apple(text));
  let body = v_flex()
    .gap_3()
    .child(h_flex().gap_2().items_center().child(div().text_xs().text_color(muted).child(apple("Issuer ID"))).child(highlighted(
      "issuer-chip",
      1,
      "69a6de70-03db-47e3-…",
      cx,
    )))
    .child(
      v_flex()
        .gap_2()
        .p_2()
        .rounded_md()
        .border_1()
        .border_color(theme.border)
        .child(h_flex().gap_2().child(header("Name")).child(header("Key ID")).child(header("Role")))
        .child(
          h_flex()
            .gap_2()
            .items_center()
            .child(div().flex_1().text_xs().font_weight(FontWeight::SEMIBOLD).child(apple("AppForge")))
            .child(div().flex_1().child(highlighted("key-chip", 2, "ABC123DEFG", cx)))
            .child(div().flex_1().text_xs().child(apple("App Manager"))),
        ),
    );
  browser("appstoreconnect.apple.com", div().p_3().child(body), cx)
}

/// Step 3: the downloaded file goes into AppForge. `chosen` is the name of the file once it is picked.
pub fn add_key_file(chosen: Option<&str>, language: Language, cx: &App) -> impl IntoElement {
  let theme = cx.theme();
  let color = theme.primary;
  let tile = |icon: IconName| {
    div().size_9().rounded_md().flex().items_center().justify_center().bg(theme.secondary).text_color(theme.foreground).child(Icon::new(icon))
  };
  let file_name: SharedString = chosen.map(|name| SharedString::from(name.to_owned())).unwrap_or_else(|| apple("AuthKey_ABC123DEFG.p8"));
  let mut file = h_flex()
    .gap_2()
    .items_center()
    .px_3()
    .py_2()
    .rounded_lg()
    .border_1()
    .child(tile(IconName::FileText))
    .child(div().text_sm().child(file_name.clone()));
  file = if chosen.is_some() { file.border_color(theme.success) } else { file.border_dashed().border_color(theme.muted_foreground) };
  let arrow = pulse("file-arrow", 0.2, 1., div().text_color(color).child(Icon::new(IconName::ArrowRight)));
  let mut target = v_flex()
    .gap_1()
    .items_center()
    .child(tile(IconName::LayoutDashboard))
    .child(div().text_xs().text_color(theme.muted_foreground).child("AppForge"));
  if chosen.is_some() {
    target = target.child(reveal(
      SharedString::from(format!("file-ok-{file_name}")),
      h_flex()
        .gap_1()
        .items_center()
        .text_xs()
        .text_color(theme.success)
        .child(Icon::new(IconName::CircleCheck).small())
        .child(tr(language, "Ready")),
    ));
  }
  h_flex().w_full().gap_3().items_center().justify_center().p_3().rounded_lg().bg(theme.secondary.opacity(0.5)).child(file).child(arrow).child(target)
}
