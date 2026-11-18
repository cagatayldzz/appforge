use crate::{
  i18n::tr,
  widgets::SIDEBAR_WIDTH,
  workspace::{Load, Workspace},
};
use gpui_kit::{
  component::{
    avatar::Avatar,
    button::{Button, ButtonVariants as _},
    h_flex,
    label::Label,
    separator::Separator,
    sidebar::{Sidebar, SidebarGroup, SidebarMenu, SidebarMenuItem},
    tag::Tag,
    v_flex, ActiveTheme, Icon, IconName, Selectable as _, Sizable as _,
  },
  *,
};

impl Workspace {
  pub(crate) fn render_sidebar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
    let theme = cx.theme().clone();
    let l = self.settings.language;
    let muted = theme.muted_foreground;
    let apps = match &self.apps {
      Load::Ready(apps) => apps.clone(),
      _ => Vec::new(),
    };
    let items = apps.into_iter().map(|app| {
      let id = app.id.clone();
      let selected = !self.show_settings && self.selected.as_ref() == Some(&id);
      let listener = cx.listener(move |this, _, window, cx| this.select_app(id.clone(), window, cx));
      SidebarMenuItem::new(app.name.clone())
        .icon(Icon::new(IconName::LayoutDashboard))
        .active(selected)
        .on_click(move |event, window, cx| listener(event, window, cx))
    });
    let workspace = cx.entity();
    let group = SidebarGroup::new(tr(l, "APP STORE"))
      .suffix(move |_, _| {
        let workspace = workspace.clone();
        Button::new("reload-apps")
          .icon(Icon::new(IconName::RefreshCw))
          .ghost()
          .xsmall()
          .tooltip(tr(l, "Reload apps"))
          .on_click(move |_, window, cx| workspace.update(cx, |this, cx| this.load_apps(window, cx)))
      })
      .child(SidebarMenu::new().children(items));
    // The header and footer are plain containers: Kit's `SidebarHeader` and `SidebarFooter` light up on hover.
    let header = h_flex()
      .w_full()
      .gap_3()
      .items_center()
      .px_4()
      .pt_5()
      .pb_3()
      .child(Avatar::new().placeholder(Icon::new(IconName::LayoutDashboard)))
      .child(div().text_lg().font_weight(FontWeight::SEMIBOLD).child("AppForge"));
    let footer = v_flex()
      .w_full()
      .gap_3()
      .p_4()
      .child(Separator::horizontal())
      .child(
        h_flex()
          .justify_between()
          .items_center()
          .child(Label::new(tr(l, "Google Play")).text_sm())
          .child(Tag::secondary().small().child(tr(l, "Soon"))),
      )
      .child(
        h_flex()
          .gap_2()
          .child(
            Button::new("open-settings")
              .icon(Icon::new(IconName::Settings))
              .label(tr(l, "Settings"))
              .outline()
              .small()
              .flex_1()
              .selected(self.show_settings)
              .on_click(cx.listener(|this, _, _, cx| {
                this.show_settings = true;
                cx.notify();
              })),
          )
          .child(
            Button::new("sign-out").label(tr(l, "Sign out")).ghost().small().on_click(cx.listener(|this, _, window, cx| this.sign_out(window, cx))),
          ),
      );
    let loading = matches!(self.apps, Load::Loading).then(|| div().px_4().text_sm().text_color(muted).child(tr(l, "Loading apps…")));
    Sidebar::<SidebarGroup<SidebarMenu>>::new("app-sidebar")
      .collapsible(false)
      .w(px(SIDEBAR_WIDTH))
      .header(v_flex().child(header).children(loading))
      .footer(footer)
      .child(group)
  }
}
