//! The frame of the first-run screens: a fixed sidebar with the steps on the left, the current step on the right.

use crate::{i18n::tr, widgets::SIDEBAR_WIDTH, workspace::Workspace};
use gpui_kit::{
  component::{
    h_flex,
    label::Label,
    scroll::ScrollableElement as _,
    stepper::{Stepper, StepperItem},
    tag::Tag,
    v_flex, ActiveTheme, Icon, IconName, Sizable as _,
  },
  *,
};

/// The widest the step content grows; longer lines are hard to read.
pub const SETUP_CONTENT_WIDTH: f32 = 920.;

/// The steps of the first run: the welcome screen, then the three steps of the sign-in guide.
const STEPS: [(&str, &str); 4] =
  [("Welcome", "What AppForge does"), ("Create key", "On Apple's website"), ("Copy IDs", "Issuer ID and Key ID"), ("Sign in", "Add the key file")];

impl Workspace {
  /// Which of `STEPS` is showing.
  pub(crate) fn setup_step(&self) -> usize {
    if self.settings.onboarded {
      self.login.step + 1
    } else {
      0
    }
  }

  /// `content` next to the sidebar, with `current` (an index of `STEPS`) highlighted.
  pub(crate) fn render_setup(&mut self, current: usize, content: impl IntoElement, cx: &mut Context<Self>) -> impl IntoElement {
    let theme = cx.theme().clone();
    let l = self.settings.language;
    let items = STEPS.map(|(title, hint)| {
      StepperItem::new().child(
        v_flex()
          .gap_0p5()
          .child(div().font_weight(FontWeight::SEMIBOLD).child(tr(l, title)))
          .child(div().text_xs().text_color(theme.muted_foreground).child(tr(l, hint))),
      )
    });
    // Going back is allowed; going forward uses each screen's own button so every step is checked.
    let stepper = Stepper::new("setup-steps").vertical().selected_index(current).items(items).on_click(cx.listener(|this, target: &usize, _, cx| {
      if *target > 0 && *target <= this.setup_step() {
        this.go_to_step(*target - 1, cx);
      }
    }));
    let brand = h_flex()
      .gap_3()
      .items_center()
      .child(
        div()
          .size_9()
          .rounded_lg()
          .flex()
          .items_center()
          .justify_center()
          .bg(theme.primary)
          .text_color(theme.primary_foreground)
          .child(Icon::new(IconName::LayoutDashboard)),
      )
      .child(
        v_flex()
          .child(div().text_lg().font_weight(FontWeight::BOLD).child("AppForge"))
          .child(div().text_xs().text_color(theme.muted_foreground).child(tr(l, "Connect to App Store Connect"))),
      );
    let picker = self.language_picker("setup-language", true, cx);
    let sidebar = v_flex()
      .w(px(SIDEBAR_WIDTH))
      .flex_shrink_0()
      .h_full()
      .p_4()
      .gap_6()
      .bg(theme.sidebar)
      .border_r_1()
      .border_color(theme.border)
      .child(brand)
      .child(stepper)
      .child(div().flex_1())
      .child(v_flex().gap_2().child(Label::new(tr(l, "Language"))).child(picker))
      .child(
        h_flex()
          .gap_2()
          .items_center()
          .text_sm()
          .text_color(theme.muted_foreground)
          .child(tr(l, "Google Play"))
          .child(Tag::secondary().small().child(tr(l, "Coming soon"))),
      );
    h_flex().flex_1().min_h_0().child(sidebar).child(v_flex().id("setup-content").flex_1().min_w_0().min_h_0().child(content).overflow_y_scrollbar())
  }
}
