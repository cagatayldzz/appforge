use crate::{
  forms::read_input,
  guide_art::{self, reveal},
  i18n::tr,
  workspace::Workspace,
};
use gpui_kit::{
  component::{
    alert::Alert,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::Input,
    label::Label,
    scroll::ScrollableElement as _,
    stepper::{Stepper, StepperItem},
    tag::Tag,
    v_flex, ActiveTheme, Disableable as _, Icon, IconName, Sizable as _,
  },
  *,
};

/// Width of the sign-in guide.
const LOGIN_WIDTH: f32 = 520.;
const API_KEYS_URL: &str = "https://appstoreconnect.apple.com/access/integrations/api";
const STEP_COUNT: usize = 3;

impl Workspace {
  /// Moves the guide to `step`. Going forward needs the Issuer ID and Key ID from step 2 on.
  fn go_to_step(&mut self, step: usize, cx: &mut Context<Self>) {
    let l = self.settings.language;
    let step = step.min(STEP_COUNT - 1);
    if step >= 2 && (read_input(&self.login.issuer, cx).is_empty() || read_input(&self.login.key_id, cx).is_empty()) {
      self.login.step = 1;
      self.login.error = Some(tr(l, "Enter both the Issuer ID and the Key ID to continue."));
    } else {
      self.login.step = step;
      self.login.error = None;
    }
    cx.notify();
  }

  fn numbered(&self, number: usize, text: impl IntoElement, cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    h_flex()
      .gap_3()
      .items_start()
      .child(
        div()
          .flex_shrink_0()
          .size_5()
          .rounded_full()
          .flex()
          .items_center()
          .justify_center()
          .text_xs()
          .font_weight(FontWeight::SEMIBOLD)
          .bg(theme.secondary)
          .text_color(theme.secondary_foreground)
          .child(number.to_string()),
      )
      .child(div().flex_1().min_w_0().text_sm().child(text))
  }

  fn step_create_key(&self, cx: &mut Context<Self>) -> Div {
    let l = self.settings.language;
    let hint = |text: SharedString| div().text_color(cx.theme().muted_foreground).child(text);
    v_flex()
      .gap_3()
      .child(guide_art::create_key(l, cx))
      .child(div().font_weight(FontWeight::SEMIBOLD).child(tr(l, "Create an API key on Apple's website")))
      .child(hint(tr(l, "AppForge talks to App Store Connect with a key that you create once. It takes about two minutes.")).text_sm())
      .child(self.numbered(1, tr(l, "Open the page below and sign in with your Apple Developer account."), cx))
      .child(self.numbered(2, tr(l, "Click the + button to add a new key."), cx))
      .child(self.numbered(3, tr(l, "Name it \"AppForge\", choose the role App Manager and click Generate."), cx))
      .child(self.numbered(4, tr(l, "Click \"Download API Key\" next to the new key. Apple lets you download it only once, so keep the file."), cx))
      .child(
        h_flex().pt_1().child(
          Button::new("create-key")
            .icon(Icon::new(IconName::ExternalLink))
            .label(tr(l, "Open App Store Connect"))
            .outline()
            .on_click(|_, _, cx| cx.open_url(API_KEYS_URL)),
        ),
      )
  }

  fn step_ids(&self, cx: &mut Context<Self>) -> Div {
    let l = self.settings.language;
    let hint = |text: SharedString| div().text_xs().text_color(cx.theme().muted_foreground).child(text);
    v_flex()
      .gap_4()
      .child(guide_art::copy_ids(cx))
      .child(div().font_weight(FontWeight::SEMIBOLD).child(tr(l, "Copy two values from the same page")))
      .child(
        v_flex()
          .gap_1()
          .child(Label::new(tr(l, "Issuer ID")))
          .child(Input::new(&self.login.issuer))
          .child(hint(tr(l, "A long code with dashes, shown at the top of the key list, above the keys."))),
      )
      .child(
        v_flex()
          .gap_1()
          .child(Label::new(tr(l, "Key ID")))
          .child(Input::new(&self.login.key_id))
          .child(hint(tr(l, "A short code of 10 letters and digits, in the \"Key ID\" column of your AppForge key."))),
      )
  }

  fn step_private_key(&self, cx: &mut Context<Self>) -> Div {
    let l = self.settings.language;
    let theme = cx.theme().clone();
    let (key_label, chosen): (SharedString, bool) = match &self.login.key {
      Some(key) => (key.file_name.clone().into(), true),
      None => (tr(l, "No file chosen"), false),
    };
    let art = guide_art::add_key_file(self.login.key.as_ref().map(|key| key.file_name.as_str()), l, cx);
    v_flex().gap_4().child(art).child(div().font_weight(FontWeight::SEMIBOLD).child(tr(l, "Add the key file you downloaded"))).child(
      v_flex()
        .gap_2()
        .child(Label::new(tr(l, "Private key (.p8)")))
        .child(
          h_flex()
            .gap_2()
            .items_center()
            .child(
              Button::new("choose-key")
                .icon(Icon::new(if chosen { IconName::Check } else { IconName::Plus }))
                .label(if chosen { tr(l, "Choose another file") } else { tr(l, "Choose .p8 file") })
                .outline()
                .on_click(cx.listener(|this, _, window, cx| this.choose_key_file(window, cx))),
            )
            .child(div().text_sm().text_color(theme.muted_foreground).child(key_label)),
        )
        .child(div().text_xs().text_color(theme.muted_foreground).child(tr(
          l,
          "The file is named AuthKey_<Key ID>.p8, usually in your Downloads folder. It is stored in the macOS Keychain and never leaves this Mac.",
        ))),
    )
  }

  pub(crate) fn render_login(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
    let theme = cx.theme().clone();
    let l = self.settings.language;
    let step = self.login.step.min(STEP_COUNT - 1);
    let body = match step {
      0 => self.step_create_key(cx),
      1 => self.step_ids(cx),
      _ => self.step_private_key(cx),
    };

    let stepper = Stepper::new("login-steps")
      .selected_index(step)
      .items([StepperItem::new().child(tr(l, "Create key")), StepperItem::new().child(tr(l, "Copy IDs")), StepperItem::new().child(tr(l, "Sign in"))])
      .on_click(cx.listener(|this, target: &usize, _, cx| {
        // Going back is always allowed; going forward uses the Continue button so each step is checked.
        if *target <= this.login.step {
          this.go_to_step(*target, cx);
        }
      }));

    let mut nav = h_flex().gap_2().justify_end().pt_2();
    if step > 0 {
      nav =
        nav.child(Button::new("login-back").label(tr(l, "Back")).ghost().on_click(cx.listener(move |this, _, _, cx| this.go_to_step(step - 1, cx))));
    }
    nav = if step + 1 < STEP_COUNT {
      let label = if step == 0 { tr(l, "I have the key file") } else { tr(l, "Continue") };
      nav.child(Button::new("login-next").label(label).primary().on_click(cx.listener(move |this, _, _, cx| this.go_to_step(step + 1, cx))))
    } else {
      nav.child(
        Button::new("sign-in")
          .label(tr(l, "Sign in"))
          .primary()
          .loading(self.login.busy)
          .disabled(self.login.busy || self.settings_locked)
          .on_click(cx.listener(|this, _, window, cx| this.sign_in(window, cx))),
      )
    };

    let mut card = v_flex()
      .gap_4()
      .p_5()
      .rounded_lg()
      .border_1()
      .border_color(theme.border)
      .bg(theme.secondary.opacity(0.4))
      .child(reveal(SharedString::from(format!("login-step-{step}")), body));
    if let Some(error) = &self.login.error {
      card = card.child(Alert::error("login-error", error.clone()));
    }
    card = card.child(nav);

    let content = v_flex()
      .w(px(LOGIN_WIDTH))
      .max_w_full()
      .gap_5()
      .py_8()
      .my_auto()
      .child(
        v_flex().gap_1().child(div().text_2xl().font_weight(FontWeight::BOLD).child(tr(l, "Connect to App Store Connect"))).child(
          div()
            .text_sm()
            .text_color(theme.muted_foreground)
            .child(tr(l, "Follow three short steps. Apple does not allow password sign-in for tools, so AppForge uses an API key instead.")),
        ),
      )
      .child(stepper)
      .child(card)
      .child(
        h_flex()
          .gap_2()
          .items_center()
          .text_sm()
          .text_color(theme.muted_foreground)
          .child(tr(l, "Google Play"))
          .child(Tag::secondary().small().child(tr(l, "Coming soon"))),
      );

    v_flex().id("login").flex_1().min_h_0().items_center().px_8().child(content).overflow_y_scrollbar()
  }
}
