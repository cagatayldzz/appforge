//! The first screen: what AppForge does and the language, before the sign-in guide.

use crate::{
  guide_art::{reveal, reveal_after},
  i18n::tr,
  workspace::Workspace,
};
use appforge_storage::Language;
use gpui_kit::{
  component::{
    button::{Button, ButtonVariants as _},
    h_flex,
    scroll::ScrollableElement as _,
    v_flex, ActiveTheme, Icon, IconName, Selectable as _, Sizable as _,
  },
  *,
};

/// Width of the welcome content.
const WELCOME_WIDTH: f32 = 520.;

impl Workspace {
  pub fn set_language(&mut self, language: Language, window: &mut Window, cx: &mut Context<Self>) {
    if self.settings.language == language {
      return;
    }
    self.settings.language = language;
    self.save_settings(window, cx);
    cx.notify();
  }

  /// One button per language. `id` keeps the buttons of different screens apart.
  pub(crate) fn language_picker(&mut self, id: &'static str, cx: &mut Context<Self>) -> impl IntoElement {
    let current = self.settings.language;
    h_flex().gap_2().children(Language::ALL.iter().map(|language| {
      let language = *language;
      Button::new(SharedString::from(format!("{id}-{}", language.native_name())))
        .icon(Icon::new(IconName::Globe))
        .label(language.native_name())
        .outline()
        .selected(current == language)
        .on_click(cx.listener(move |this, _, window, cx| this.set_language(language, window, cx)))
    }))
  }

  fn finish_welcome(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    self.settings.onboarded = true;
    self.save_settings(window, cx);
    cx.notify();
  }

  pub(crate) fn render_welcome(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
    let theme = cx.theme().clone();
    let l = self.settings.language;
    let feature = |icon: IconName, title: SharedString, text: SharedString| {
      h_flex()
        .gap_3()
        .items_start()
        .p_3()
        .rounded_lg()
        .border_1()
        .border_color(theme.border)
        .child(
          div()
            .flex_shrink_0()
            .size_9()
            .rounded_md()
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.secondary)
            .text_color(theme.primary)
            .child(Icon::new(icon)),
        )
        .child(
          v_flex()
            .flex_1()
            .min_w_0()
            .gap_0p5()
            .child(div().font_weight(FontWeight::SEMIBOLD).child(title))
            .child(div().text_sm().text_color(theme.muted_foreground).child(text)),
        )
    };
    let hero = v_flex()
      .gap_3()
      .items_center()
      .text_center()
      .child(
        div()
          .size_16()
          .rounded_xl()
          .flex()
          .items_center()
          .justify_center()
          .bg(theme.primary)
          .text_color(theme.primary_foreground)
          .child(Icon::new(IconName::LayoutDashboard).large()),
      )
      .child(div().text_3xl().font_weight(FontWeight::BOLD).child(tr(l, "Welcome to AppForge")))
      .child(div().text_color(theme.muted_foreground).child(tr(l, "Get your App Store apps ready for review, from one calm window.")));
    let features = v_flex()
      .gap_2()
      .child(reveal_after(
        "welcome-feature-0",
        250,
        feature(
          IconName::FileText,
          tr(l, "Edit your listing"),
          tr(l, "Change names, descriptions, keywords and review notes, and save them straight to App Store Connect."),
        ),
      ))
      .child(reveal_after(
        "welcome-feature-1",
        400,
        feature(
          IconName::CircleCheck,
          tr(l, "Check review readiness"),
          tr(l, "See what could get your app rejected, based on Apple's guidelines, before you submit."),
        ),
      ))
      .child(reveal_after(
        "welcome-feature-2",
        550,
        feature(
          IconName::Bot,
          tr(l, "Ask an AI to double-check"),
          tr(l, "Optional. Use your own Claude or OpenAI key; your keys stay in the macOS Keychain."),
        ),
      ));
    let picker = self.language_picker("welcome-language", cx);
    let content = v_flex()
      .w(px(WELCOME_WIDTH))
      .max_w_full()
      .gap_6()
      .py_8()
      .my_auto()
      .child(reveal("welcome-hero", hero))
      .child(features)
      .child(v_flex().gap_2().child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child(tr(l, "Language"))).child(picker))
      .child(
        v_flex()
          .gap_2()
          .items_center()
          .child(
            Button::new("welcome-start")
              .label(tr(l, "Get started"))
              .primary()
              .large()
              .w_full()
              .on_click(cx.listener(|this, _, window, cx| this.finish_welcome(window, cx))),
          )
          .child(div().text_xs().text_color(theme.muted_foreground).child(tr(l, "Connecting your account takes about two minutes."))),
      );
    v_flex().id("welcome").flex_1().min_h_0().items_center().px_8().child(content).overflow_y_scrollbar()
  }
}
