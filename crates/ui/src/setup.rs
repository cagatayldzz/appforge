//! The frame of the first-run screens: a fixed sidebar on the left; on the right the progress above the current step,
//! and the actions pinned at the bottom so they never scroll out of sight.

use crate::{
  i18n::{tr, trf},
  theme::colors,
  widgets::SIDEBAR_WIDTH,
  workspace::Workspace,
};
use gpui_kit::{
  component::{
    h_flex,
    label::Label,
    progress::Progress,
    scroll::ScrollableElement as _,
    select::Select,
    separator::Separator,
    sidebar::{Sidebar, SidebarMenu},
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
    let palette = colors(cx);
    let (sidebar_actions, main_actions) = if current == 0 { (actions, None) } else { (None, actions) };
    // The header and footer are plain containers: Kit's `SidebarHeader` and `SidebarFooter` light up on hover.
    let brand = h_flex()
      .gap_3()
      .items_center()
      .child(
        div()
          .size_10()
          .rounded_md()
          .bg(palette.selected)
          .flex()
          .items_center()
          .justify_center()
          .child(Icon::new(IconName::LayoutDashboard).text_color(palette.text)),
      )
      .child(div().text_lg().font_weight(FontWeight::SEMIBOLD).child("AppForge"));
    // Why the guide asks for an API key, and where the key goes: shown with the sign-in steps.
    let note = |icon: IconName, title: SharedString, text: SharedString| {
      v_flex()
        .gap_1p5()
        .child(h_flex().gap_2().items_center().child(Icon::new(icon).small()).child(Label::new(title).text_sm()))
        .child(div().text_xs().text_color(muted).child(text))
    };
    let notes = (current > 0).then(|| {
      v_flex().gap_5().child(note(IconName::Info, tr(l, "whyAnApiKey"), tr(l, "appleDoesNotAllow"))).child(note(
        IconName::HardDrive,
        tr(l, "staysOnThisMac"),
        tr(l, "yourKeyIsStored"),
      ))
    });
    let progress = v_flex()
      .w_full()
      .gap_2()
      .child(div().text_sm().font_weight(FontWeight::MEDIUM).text_color(muted).child(trf(
        l,
        "stepOf",
        &[&(current + 1).to_string(), &STEP_TOTAL.to_string()],
      )))
      .child(Progress::new("setup-progress").xsmall().value((current + 1) as f32 / STEP_TOTAL as f32 * 100.));
    let (sidebar_progress, main_progress) = if current == 0 {
      (Some(progress), None)
    } else {
      (None, Some(div().w_full().max_w(px(SETUP_CONTENT_WIDTH)).px_10().pt_8().pb_3().child(progress)))
    };
    let header = v_flex().w_full().gap_5().p_4().child(brand).child(Separator::horizontal()).children(sidebar_progress).children(notes);
    let footer = v_flex()
      .w_full()
      .gap_3()
      .p_4()
      .child(Separator::horizontal())
      .child(v_flex().gap_1p5().child(div().text_xs().text_color(muted).child(tr(l, "language"))).child(Select::new(&self.language_select).w_full()))
      .children(sidebar_actions);
    let sidebar = div()
      .w(px(SIDEBAR_WIDTH))
      .flex_shrink_0()
      .h_full()
      .bg(palette.surface)
      .border_r_1()
      .border_color(palette.border)
      .child(Sidebar::<SidebarMenu>::new("setup-sidebar").collapsible(false).w_full().header(header).footer(footer));
    let bar = main_actions.map(|actions| {
      v_flex()
        .w_full()
        .flex_shrink_0()
        .items_center()
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
        .child(v_flex().id("setup-content").flex_1().min_h_0().items_center().overflow_y_scrollbar().children(main_progress).child(content))
        .children(bar),
    )
  }
}
