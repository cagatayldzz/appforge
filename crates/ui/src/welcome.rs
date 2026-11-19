//! The first screen: what AppForge does and the language, before the sign-in guide.

use crate::{
  i18n::tr,
  theme::colors,
  widgets::{centered, section},
  workspace::Workspace,
};
use appforge_storage::Language;
use gpui_kit::{
  component::{
    button::{Button, ButtonVariants as _},
    h_flex, v_flex, Icon, IconName, Sizable as _,
  },
  *,
};

impl Workspace {
  pub fn set_language(&mut self, language: Language, window: &mut Window, cx: &mut Context<Self>) {
    if self.settings.language == language {
      return;
    }
    self.settings.language = language;
    self.save_settings(window, cx);
    cx.notify();
  }

  fn finish_welcome(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    self.settings.onboarded = true;
    self.save_settings(window, cx);
    cx.notify();
  }

  pub(crate) fn render_welcome(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
    let palette = colors(cx);
    let l = self.settings.language;
    let feature = |icon, title, text| {
      section(
        title,
        h_flex()
          .gap_3()
          .items_start()
          .child(
            div()
              .size_8()
              .flex_none()
              .rounded_md()
              .bg(palette.surface_alt)
              .flex()
              .items_center()
              .justify_center()
              .child(Icon::new(icon).small().text_color(palette.text_secondary)),
          )
          .child(div().min_w_0().text_sm().text_color(palette.text_secondary).child(text)),
      )
    };
    let content =
      centered(
        v_flex()
          .pt_6()
          .pb_6()
          .gap_6()
          .child(v_flex().gap_2().child(div().text_xl().font_weight(FontWeight::SEMIBOLD).child(tr(l, "Welcome to AppForge"))).child(
            div().text_sm().text_color(palette.text_secondary).child(tr(l, "Get your App Store apps ready for review, from one calm window.")),
          ))
          .child(feature(
            IconName::FileText,
            tr(l, "Edit your listing"),
            tr(l, "Change names, descriptions, keywords and review notes, and save them straight to App Store Connect."),
          ))
          .child(feature(
            IconName::CircleCheck,
            tr(l, "Check review readiness"),
            tr(l, "See what could get your app rejected, based on Apple's guidelines, before you submit."),
          ))
          .child(feature(
            IconName::Bot,
            tr(l, "Ask an AI to double-check"),
            tr(l, "Optional. Use your own Claude or OpenAI key; your keys stay in the macOS Keychain."),
          )),
      );
    let start = v_flex().gap_2().child(
      Button::new("welcome-start")
        .label(tr(l, "Get started"))
        .primary()
        .w_full()
        .on_click(cx.listener(|this, _, window, cx| this.finish_welcome(window, cx))),
    );
    self.render_setup(0, content, Some(start), cx)
  }
}
