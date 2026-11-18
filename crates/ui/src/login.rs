use crate::workspace::Workspace;
use gpui_kit::{
  component::{
    alert::Alert,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::Input,
    label::Label,
    tag::Tag,
    v_flex, ActiveTheme, Disableable as _, Icon, IconName, Sizable as _,
  },
  *,
};

/// Width of the sign-in card.
const LOGIN_WIDTH: f32 = 440.;
const API_KEYS_URL: &str = "https://appstoreconnect.apple.com/access/integrations/api";

impl Workspace {
  pub(crate) fn render_login(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
    let theme = cx.theme().clone();
    let key_label: SharedString = match &self.login.key {
      Some(key) => key.file_name.clone().into(),
      None => "No file chosen".into(),
    };
    let mut card = v_flex()
      .w(px(LOGIN_WIDTH))
      .gap_4()
      .child(
        v_flex()
          .gap_1()
          .child(div().text_2xl().font_weight(FontWeight::BOLD).child("Connect to App Store Connect"))
          .child(div().text_sm().text_color(theme.muted_foreground).child(
            "AppForge signs in with an App Store Connect API key. Apple does not allow password sign-in for tools. Create a key with the App Manager role under Users and Access > Integrations.",
          )),
      )
      .child(v_flex().gap_1().child(Label::new("Issuer ID")).child(Input::new(&self.login.issuer)))
      .child(v_flex().gap_1().child(Label::new("Key ID")).child(Input::new(&self.login.key_id)))
      .child(
        v_flex().gap_1().child(Label::new("Private key (.p8)")).child(
          h_flex()
            .gap_2()
            .items_center()
            .child(Button::new("choose-key").icon(Icon::new(IconName::Plus)).label("Choose .p8 file").outline().on_click(cx.listener(|this, _, window, cx| this.choose_key_file(window, cx))))
            .child(div().text_sm().text_color(theme.muted_foreground).child(key_label)),
        ),
      );
    if let Some(error) = &self.login.error {
      card = card.child(Alert::error("login-error", error.clone()));
    }
    card
      .child(
        h_flex()
          .gap_2()
          .child(
            Button::new("sign-in")
              .label("Sign in")
              .primary()
              .loading(self.login.busy)
              .disabled(self.login.busy || self.settings_locked)
              .on_click(cx.listener(|this, _, window, cx| this.sign_in(window, cx))),
          )
          .child(Button::new("create-key").label("Create a key").ghost().on_click(|_, _, cx| cx.open_url(API_KEYS_URL))),
      )
      .child(
        h_flex()
          .gap_2()
          .items_center()
          .pt_4()
          .text_sm()
          .text_color(theme.muted_foreground)
          .child("Google Play")
          .child(Tag::secondary().small().child("Coming soon")),
      )
      .into_login_center()
  }
}

trait CenterLogin: IntoElement + Sized {
  fn into_login_center(self) -> Div {
    div().flex_1().flex().items_center().justify_center().p_8().child(self)
  }
}

impl<T: IntoElement> CenterLogin for T {}
