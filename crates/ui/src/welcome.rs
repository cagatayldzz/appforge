//! The first screen: what AppForge does and the language, before the sign-in guide.

use crate::{
  guide_art::{reveal, reveal_after},
  i18n::tr,
  setup::SETUP_CONTENT_WIDTH,
  workspace::Workspace,
};
use appforge_storage::Language;
use gpui_kit::{
  component::{
    avatar::Avatar,
    button::{Button, ButtonVariants as _},
    group_box::{GroupBox, GroupBoxVariants as _},
    h_flex,
    label::Label,
    v_flex, ActiveTheme, Icon, IconName, Sizable as _,
  },
  *,
};

/// The three feature cards sit in one row when there is room, and wrap when not.
const CARD_MIN_WIDTH: f32 = 240.;

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
    let theme = cx.theme().clone();
    let l = self.settings.language;
    let feature = |icon: IconName, title: SharedString, text: SharedString| {
      div().flex_1().min_w(px(CARD_MIN_WIDTH)).child(
        GroupBox::new()
          .outline()
          .title(h_flex().gap_2().items_center().child(Icon::new(icon)).child(Label::new(title)))
          .child(div().text_sm().text_color(theme.muted_foreground).child(text)),
      )
    };
    let hero = h_flex().gap_4().items_center().child(Avatar::new().placeholder(Icon::new(IconName::LayoutDashboard)).large()).child(
      v_flex()
        .gap_1()
        .child(div().text_3xl().font_weight(FontWeight::BOLD).child(tr(l, "Welcome to AppForge")))
        .child(div().text_color(theme.muted_foreground).child(tr(l, "Get your App Store apps ready for review, from one calm window."))),
    );
    let features = h_flex()
      .flex_wrap()
      .gap_3()
      .items_stretch()
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
    let start = h_flex()
      .gap_4()
      .items_center()
      .child(
        Button::new("welcome-start")
          .label(tr(l, "Get started"))
          .primary()
          .large()
          .on_click(cx.listener(|this, _, window, cx| this.finish_welcome(window, cx))),
      )
      .child(div().text_sm().text_color(theme.muted_foreground).child(tr(l, "Connecting your account takes about two minutes.")));
    let content =
      v_flex().w_full().max_w(px(SETUP_CONTENT_WIDTH)).px_10().pt_2().pb_8().gap_8().child(reveal("welcome-hero", hero)).child(features).child(start);
    self.render_setup(0, content, None, cx)
  }
}
