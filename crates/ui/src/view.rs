//! `Render for Workspace`: the title band and which screen shows.

use crate::{
  i18n::{tr, trf},
  theme::{colors, FONT_SIZE, FONT_SIZE_SM},
  workspace::{Load, Workspace},
};
use gpui_kit::{
  component::{
    button::{Button, ButtonVariants as _},
    h_flex,
    spinner::Spinner,
    v_flex, ActiveTheme, Icon, IconName, Sizable as _, TitleBar,
  },
  *,
};

impl Render for Workspace {
  fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let theme = cx.theme().clone();
    let l = self.settings.language;
    let body = if self.starting {
      v_flex()
        .flex_1()
        .items_center()
        .justify_center()
        .gap_2()
        .child(Spinner::new())
        .child(div().text_sm().text_color(theme.muted_foreground).child(tr(l, "openingYourKeychain")))
        .into_any_element()
    } else if self.client.is_none() && !self.settings.onboarded && !self.settings_locked {
      self.render_welcome(cx).into_any_element()
    } else if self.client.is_none() {
      self.render_login(cx).into_any_element()
    } else {
      // The grid of all apps takes the whole width; every other page has the sidebar next to it.
      let sidebar = (!self.showing_all_apps()).then(|| self.render_sidebar(window, cx));
      let main = v_flex().flex_1().h_full().min_w_0().min_h_0().overflow_hidden().child(self.render_main(window, cx));
      h_flex().flex_1().min_h_0().child(self.render_rail(cx)).children(sidebar).child(main).into_any_element()
    };
    let c = colors(cx);
    // After a jump from another page, a way back to it sits in the title bar, next to the window buttons.
    let back = self.back_tab.filter(|_| self.client.is_some() && !self.show_settings && !self.showing_all_apps()).map(|origin| {
      Button::new("back-to-page")
        .icon(Icon::new(IconName::ArrowLeft))
        .label(tr(l, origin.label()))
        .ghost()
        .small()
        .tooltip(trf(l, "backTo", &[&tr(l, origin.label())]))
        .on_click(cx.listener(|this, _, window, cx| this.go_back(window, cx)))
    });
    v_flex()
      .size_full()
      .bg(theme.background)
      .text_color(theme.foreground)
      .text_size(FONT_SIZE)
      // The title bar is the drag area for moving the window.
      .child(
        TitleBar::new()
          .bg(c.rail)
          .border_b_1()
          .border_color(c.border)
          .children(back)
          // The name sits in the middle of the band; the traffic lights stay at its left edge.
          .child(
            h_flex().flex_1().justify_center().text_size(FONT_SIZE_SM).font_weight(FontWeight::MEDIUM).text_color(c.text_secondary).child("AppForge"),
          ),
      )
      .child(body)
  }
}

impl Workspace {
  /// Whether the main area shows the grid of all apps: asked for with the Apps button, or no app is open yet.
  pub(crate) fn showing_all_apps(&self) -> bool {
    !self.show_settings && matches!(&self.apps, Load::Ready(apps) if !apps.is_empty()) && (self.show_all_apps || self.selected_app().is_none())
  }

  fn render_main(&mut self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
    let l = self.settings.language;
    let content = if self.show_settings {
      self.render_settings(crate::widgets::has_room_for_columns(window), cx).into_any_element()
    } else {
      match (&self.apps, self.selected_app()) {
        (Load::Ready(_), _) if self.showing_all_apps() => self.render_apps(window, cx).into_any_element(),
        (Load::Ready(_), Some(app)) => self.render_detail(app, crate::widgets::has_room_for_columns(window), cx).into_any_element(),
        (Load::Ready(apps), None) if apps.is_empty() => {
          self.render_message(tr(l, "noAppsFound"), tr(l, "thisAppStoreConnect"), cx).into_any_element()
        }
        (Load::Failed(message), _) => self.render_message(tr(l, "couldNotLoadYour"), message.clone(), cx).into_any_element(),
        (Load::Ready(_), None) => self.render_apps(window, cx).into_any_element(),
        _ => v_flex().flex_1().items_center().justify_center().child(Spinner::new()).into_any_element(),
      }
    };
    v_flex().id("main").flex_1().min_w_0().min_h_0().overflow_hidden().child(content)
  }
}
