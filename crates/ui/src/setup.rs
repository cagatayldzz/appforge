//! The frame of the first-run screens: a fixed sidebar on the left; on the right the progress above the current step,
//! and the actions pinned at the bottom so they never scroll out of sight.

use crate::{
  i18n::{tr, trf},
  widgets::SIDEBAR_WIDTH,
  workspace::Workspace,
};
use gpui_kit::{
  component::{
    avatar::Avatar,
    h_flex,
    label::Label,
    progress::Progress,
    scroll::ScrollableElement as _,
    select::Select,
    separator::Separator,
    sidebar::{Sidebar, SidebarMenu},
    tag::Tag,
    v_flex, ActiveTheme, Icon, IconName, Sizable as _,
  },
  *,
};

/// The widest the step content grows; longer lines are hard to read.
pub const SETUP_CONTENT_WIDTH: f32 = 920.;
/// How many steps the first run has: the welcome screen and the three steps of the sign-in guide.
const STEP_TOTAL: usize = 4;

impl Workspace {
  /// `content` next to the sidebar, with `current` (0 for the welcome screen, then 1 to 3) as the progress.
  /// `actions` are pinned under the content.
  pub(crate) fn render_setup(&mut self, current: usize, content: impl IntoElement, actions: Option<Div>, cx: &mut Context<Self>) -> impl IntoElement {
    let l = self.settings.language;
    let muted = cx.theme().muted_foreground;
    // The header and footer are plain containers: Kit's `SidebarHeader` and `SidebarFooter` light up on hover.
    let brand = h_flex()
      .gap_3()
      .items_center()
      .child(Avatar::new().placeholder(Icon::new(IconName::LayoutDashboard)))
      .child(div().text_lg().font_weight(FontWeight::SEMIBOLD).child("AppForge"));
    // Why the guide asks for an API key, and where the key goes: shown with the sign-in steps.
    let note = |icon: IconName, title: SharedString, text: SharedString| {
      v_flex()
        .gap_1p5()
        .child(h_flex().gap_2().items_center().child(Icon::new(icon).small()).child(Label::new(title).text_sm()))
        .child(div().text_xs().text_color(muted).child(text))
    };
    let notes = (current > 0).then(|| {
      v_flex()
        .gap_5()
        .child(note(
          IconName::Info,
          tr(l, "Why an API key?"),
          tr(l, "Apple does not allow password sign-in for tools, so AppForge uses an API key instead."),
        ))
        .child(note(
          IconName::HardDrive,
          tr(l, "Stays on this Mac"),
          tr(l, "Your key is stored in the macOS Keychain and is never sent anywhere except Apple."),
        ))
    });
    let header = v_flex().w_full().gap_6().px_4().pt_5().pb_3().child(brand).children(notes);
    let footer = v_flex()
      .w_full()
      .gap_4()
      .p_4()
      .child(Separator::horizontal())
      .child(v_flex().gap_1p5().child(div().text_xs().text_color(muted).child(tr(l, "Language"))).child(Select::new(&self.language_select).w_full()))
      .child(
        h_flex()
          .justify_between()
          .items_center()
          .child(Label::new(tr(l, "Google Play")).text_sm())
          .child(Tag::secondary().small().child(tr(l, "Coming soon"))),
      );
    let sidebar = Sidebar::<SidebarMenu>::new("setup-sidebar").collapsible(false).w(px(SIDEBAR_WIDTH)).header(header).footer(footer);
    let progress = v_flex()
      .w_full()
      .max_w(px(SETUP_CONTENT_WIDTH))
      .px_10()
      .pt_8()
      .gap_3()
      .child(div().text_sm().font_weight(FontWeight::MEDIUM).text_color(muted).child(trf(
        l,
        "Step {} of {}",
        &[&(current + 1).to_string(), &STEP_TOTAL.to_string()],
      )))
      .child(Progress::new("setup-progress").xsmall().value((current + 1) as f32 / STEP_TOTAL as f32 * 100.));
    let bar = actions.map(|actions| {
      v_flex()
        .w_full()
        .flex_shrink_0()
        .child(Separator::horizontal())
        .child(div().w_full().max_w(px(SETUP_CONTENT_WIDTH)).px_10().py_4().child(actions))
    });
    h_flex().flex_1().min_h_0().child(sidebar).child(
      v_flex()
        .flex_1()
        .h_full()
        .min_w_0()
        .min_h_0()
        .overflow_hidden()
        .child(v_flex().id("setup-content").flex_1().min_h_0().overflow_y_scrollbar().child(progress).child(content))
        .children(bar),
    )
  }
}
