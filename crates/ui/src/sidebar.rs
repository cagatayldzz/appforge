use crate::{
  i18n::tr,
  widgets::SIDEBAR_WIDTH,
  workspace::{Load, Workspace},
};
use gpui_kit::{
  component::{
    button::{Button, ButtonVariants as _},
    h_flex,
    scroll::ScrollableElement as _,
    tag::Tag,
    v_flex, ActiveTheme, Icon, IconName, Selectable as _, Sizable as _,
  },
  *,
};

impl Workspace {
  pub(crate) fn render_sidebar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
    let theme = cx.theme().clone();
    let l = self.settings.language;
    let apps: Vec<_> = match &self.apps {
      Load::Ready(apps) => apps.clone(),
      _ => Vec::new(),
    };
    let list = v_flex().id("app-list").flex_1().min_h_0().gap_1().children(apps.into_iter().map(|app| {
      let id = app.id.clone();
      let selected = !self.show_settings && self.selected.as_ref() == Some(&id);
      Button::new(SharedString::from(format!("app-{id}")))
        .label(app.name.clone())
        .ghost()
        .w_full()
        .justify_start()
        .selected(selected)
        .on_click(cx.listener(move |this, _, window, cx| this.select_app(id.clone(), window, cx)))
    }));
    let list = match &self.apps {
      Load::Loading => list.child(div().px_2().text_sm().text_color(theme.muted_foreground).child(tr(l, "Loading apps…"))),
      _ => list,
    };
    v_flex()
      .w(px(SIDEBAR_WIDTH))
      .flex_shrink_0()
      .h_full()
      .pt_4()
      .px_3()
      .pb_3()
      .gap_1()
      .bg(theme.sidebar)
      .border_r_1()
      .border_color(theme.border)
      .child(div().px_2().pb_4().text_xl().font_weight(FontWeight::BOLD).child("AppForge"))
      .child(
        h_flex()
          .px_2()
          .pb_1()
          .justify_between()
          .items_center()
          .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(theme.muted_foreground).child(tr(l, "APP STORE")))
          .child(
            Button::new("reload-apps")
              .icon(Icon::new(IconName::Redo))
              .ghost()
              .small()
              .tooltip(tr(l, "Reload apps"))
              .on_click(cx.listener(|this, _, window, cx| this.load_apps(window, cx))),
          ),
      )
      .child(list.overflow_y_scrollbar())
      .child(
        h_flex()
          .px_2()
          .py_1()
          .gap_2()
          .items_center()
          .text_sm()
          .text_color(theme.muted_foreground)
          .child(tr(l, "Google Play"))
          .child(Tag::secondary().small().child(tr(l, "Soon"))),
      )
      .child(
        Button::new("open-settings")
          .icon(Icon::new(IconName::Settings))
          .label(tr(l, "Settings"))
          .ghost()
          .w_full()
          .justify_start()
          .selected(self.show_settings)
          .on_click(cx.listener(|this, _, _, cx| {
            this.show_settings = true;
            cx.notify();
          })),
      )
      .child(
        Button::new("sign-out")
          .label(tr(l, "Sign out"))
          .ghost()
          .w_full()
          .justify_start()
          .on_click(cx.listener(|this, _, window, cx| this.sign_out(window, cx))),
      )
  }
}
