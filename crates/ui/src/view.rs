use crate::{
  i18n::tr,
  workspace::{Load, Workspace},
};
use gpui_kit::{
  component::{h_flex, spinner::Spinner, v_flex, ActiveTheme, TitleBar},
  *,
};

impl Render for Workspace {
  fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let theme = cx.theme().clone();
    let l = self.settings.language;
    let body = if self.starting {
      v_flex()
        .flex_1()
        .items_center()
        .justify_center()
        .gap_2()
        .child(Spinner::new())
        .child(div().text_sm().text_color(theme.muted_foreground).child(tr(l, "Opening your Keychain…")))
        .into_any_element()
    } else if self.client.is_none() && !self.settings.onboarded && !self.settings_locked {
      self.render_welcome(cx).into_any_element()
    } else if self.client.is_none() {
      self.render_login(cx).into_any_element()
    } else {
      h_flex().flex_1().min_h_0().child(self.render_sidebar(cx)).child(self.render_main(cx)).into_any_element()
    };
    v_flex()
      .size_full()
      .bg(theme.background)
      .text_color(theme.foreground)
      // The title bar is the drag area for moving the window.
      .child(TitleBar::new())
      .child(body)
  }
}

impl Workspace {
  fn render_main(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
    let l = self.settings.language;
    let content = if self.show_settings {
      self.render_settings(cx).into_any_element()
    } else {
      match (&self.apps, self.selected_app()) {
        (Load::Ready(_), Some(app)) => self.render_detail(app, cx).into_any_element(),
        (Load::Ready(apps), None) if apps.is_empty() => self
          .render_message(
            tr(l, "No apps found"),
            tr(l, "This App Store Connect account has no apps yet. Create one in App Store Connect, then reload."),
            cx,
          )
          .into_any_element(),
        (Load::Failed(message), _) => self.render_message(tr(l, "Could not load your apps"), message.clone(), cx).into_any_element(),
        (Load::Ready(_), None) => {
          self.render_message(tr(l, "Select an app"), tr(l, "Choose an app on the left to see and edit its information."), cx).into_any_element()
        }
        _ => v_flex().flex_1().items_center().justify_center().child(Spinner::new()).into_any_element(),
      }
    };
    v_flex().id("main").flex_1().h_full().min_w_0().min_h_0().overflow_hidden().child(content)
  }
}
