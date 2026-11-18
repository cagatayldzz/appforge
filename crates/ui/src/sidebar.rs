//! The left side of the signed-in window: the activity bar (icon buttons), the sidebar (the apps, or the pages of Settings)
//! and the status bar at the bottom of the main area.

use crate::{
  i18n::{tr, trf},
  theme::{colors, FONT_SIZE, FONT_SIZE_SM},
  widgets::{app_avatar, list_row, rail_button, state_badge, ACTIVITY_BAR_WIDTH, SIDEBAR_WIDTH},
  workspace::{Load, SettingsPage, Workspace},
};
use gpui_kit::{
  component::{
    button::{Button, ButtonVariants as _},
    h_flex,
    scroll::ScrollableElement as _,
    status_bar::StatusBar,
    tag::Tag,
    tooltip::Tooltip,
    v_flex, Icon, IconName, Sizable as _,
  },
  *,
};

impl Workspace {
  /// The strip on the far left: one avatar per app, and Settings at the bottom.
  pub(crate) fn render_rail(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
    let c = colors(cx);
    let l = self.settings.language;
    let workspace = cx.entity();
    let apps = match &self.apps {
      Load::Ready(apps) => apps.clone(),
      _ => Vec::new(),
    };
    let all_apps = {
      let workspace = workspace.clone();
      rail_button("rail-all-apps", IconName::LayoutDashboard, tr(l, "Apps"), self.show_all_apps && !self.show_settings, cx, move |_, _, cx| {
        workspace.update(cx, |this, cx| {
          this.show_settings = false;
          this.show_all_apps = true;
          cx.notify();
        })
      })
    };
    let items: Vec<AnyElement> = apps
      .into_iter()
      .map(|app| {
        let id = app.id.clone();
        let selected = !self.show_settings && !self.show_all_apps && self.selected.as_ref() == Some(&id);
        let name: SharedString = app.name.clone().into();
        div()
          .id(SharedString::from(format!("rail-app-{}", app.id)))
          .flex_none()
          .rounded_md()
          .cursor_pointer()
          .child(app_avatar(&app.id, &app.name, selected).size_10())
          .tooltip(move |window, cx| Tooltip::new(name.clone()).build(window, cx))
          .on_click(cx.listener(move |this, _, window, cx| {
            this.show_settings = false;
            this.select_app(id.clone(), window, cx)
          }))
          .into_any_element()
      })
      .collect();
    let settings = rail_button("rail-settings", IconName::Settings, tr(l, "Settings"), self.show_settings, cx, move |_, _, cx| {
      workspace.update(cx, |this, cx| {
        this.show_settings = true;
        cx.notify();
      })
    });
    v_flex()
      .w(px(ACTIVITY_BAR_WIDTH))
      .h_full()
      .flex_none()
      .items_center()
      .py_2()
      .gap_2()
      .bg(c.rail)
      .border_r_1()
      .border_color(c.border)
      .child(all_apps)
      .child(v_flex().id("rail-apps").flex_1().min_h_0().items_center().gap_2().overflow_y_scrollbar().children(items))
      .child(settings)
  }

  /// The column next to the activity bar: the apps, or the pages of Settings while Settings is open.
  pub(crate) fn render_sidebar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
    let c = colors(cx);
    let l = self.settings.language;
    let title = |text: SharedString, action: Option<AnyElement>| {
      h_flex()
        .h_10()
        .px_4()
        .flex_none()
        .items_center()
        .justify_between()
        .child(div().text_size(FONT_SIZE).font_weight(FontWeight::SEMIBOLD).child(text))
        .children(action)
        .into_any_element()
    };
    // The top of the column says what the main area shows: the pages of Settings, the app that is open, or all apps.
    let (header, rows) = if self.show_settings {
      (title(tr(l, "Settings"), None), self.settings_rows(cx))
    } else if let Some(app) = self.selected_app().filter(|_| !self.show_all_apps) {
      (self.app_identity(&app, cx), Vec::new())
    } else {
      (title(tr(l, "Apps"), Some(self.reload_button(cx).into_any_element())), Vec::new())
    };
    let footer = h_flex()
      .flex_none()
      .px_4()
      .py_3()
      .justify_between()
      .items_center()
      .border_t_1()
      .border_color(c.border)
      .text_size(FONT_SIZE_SM)
      .text_color(c.text_secondary)
      .child(tr(l, "Google Play"))
      .child(Tag::secondary().small().child(tr(l, "Soon")));
    v_flex()
      .w(px(SIDEBAR_WIDTH))
      .h_full()
      .flex_none()
      .bg(c.surface)
      .border_r_1()
      .border_color(c.border)
      .child(header)
      .child(v_flex().id("sidebar-list").flex_1().min_h_0().px_2().gap_0p5().overflow_y_scrollbar().children(rows))
      .child(footer)
  }

  fn reload_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
    let l = self.settings.language;
    Button::new("reload-apps")
      .icon(Icon::new(IconName::RefreshCw))
      .ghost()
      .xsmall()
      .tooltip(tr(l, "Reload apps"))
      .on_click(cx.listener(|this, _, window, cx| this.load_apps(window, cx)))
  }

  /// The app that is open: its avatar, name, bundle id, version and where the version stands at Apple.
  fn app_identity(&self, app: &appforge_appstore::App, cx: &mut Context<Self>) -> AnyElement {
    let c = colors(cx);
    let l = self.settings.language;
    let mut details = h_flex().flex_wrap().gap_x_2().gap_y_0p5().items_center().text_size(FONT_SIZE_SM).text_color(c.text_secondary);
    if let Some(version) = self.ready_snapshot().and_then(|snapshot| snapshot.version.as_ref()) {
      details = details.child(trf(l, "Version {}", &[&version.version_string])).child(state_badge(&version.state, &c));
    }
    h_flex()
      .w_full()
      .flex_none()
      .gap_3()
      .px_4()
      .py_4()
      .items_center()
      .border_b_1()
      .border_color(c.border)
      .child(app_avatar(&app.id, &app.name, true).size_10())
      .child(
        v_flex()
          .flex_1()
          .min_w_0()
          .gap_0p5()
          .child(div().truncate().text_size(FONT_SIZE).font_weight(FontWeight::SEMIBOLD).child(app.name.clone()))
          .child(div().truncate().text_size(FONT_SIZE_SM).text_color(c.text_secondary).child(app.bundle_id.clone()))
          .child(details),
      )
      .into_any_element()
  }

  fn settings_rows(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
    let l = self.settings.language;
    SettingsPage::ALL
      .iter()
      .map(|page| {
        let page = *page;
        list_row(SharedString::from(format!("settings-page-{}", page.label())), self.settings_page == page, cx)
          .py_2()
          .child(tr(l, page.label()))
          .on_click(cx.listener(move |this, _, _, cx| {
            this.settings_page = page;
            cx.notify();
          }))
          .into_any_element()
      })
      .collect()
  }

  /// The thin bar at the bottom of the main area: the open app on the left, the account on the right.
  pub(crate) fn render_status_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
    let c = colors(cx);
    let l = self.settings.language;
    let mut bar = StatusBar::new().bg(c.surface_alt).border_color(c.border).text_color(c.text_secondary);
    if let Some(app) = self.selected_app().filter(|_| !self.show_settings) {
      bar = bar.left(app.bundle_id.clone());
      if let Some(version) = self.ready_snapshot().and_then(|snapshot| snapshot.version.as_ref()) {
        bar = bar.left(trf(l, "Version {}", &[&version.version_string]));
      }
    }
    if let Some(account) = &self.settings.apple {
      bar = bar.right(trf(l, "Signed in with key {}.", &[&account.key_id]));
    }
    bar
  }
}
