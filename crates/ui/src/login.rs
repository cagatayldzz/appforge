//! The sign-in guide: checklist, Issuer ID and Key ID, and the .p8 file.

use crate::{
  forms::read_input,
  guide_art::{self, reveal_after, DRAWING_WIDTH, KEY_STEP_COUNT},
  i18n::{tr, trf},
  setup::SETUP_CONTENT_WIDTH,
  workspace::Workspace,
};
use appforge_storage::Language;
use gpui_kit::{
  component::{
    alert::Alert,
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    group_box::{GroupBox, GroupBoxVariants as _},
    h_flex,
    input::{Input, InputState},
    label::Label,
    separator::Separator,
    tag::Tag,
    v_flex, ActiveTheme, Disableable as _, Icon, IconName,
  },
  *,
};

const API_KEYS_URL: &str = "https://appstoreconnect.apple.com/access/integrations/api";
const STEP_COUNT: usize = 3;

impl Workspace {
  /// Moves the guide to `step`. Going forward needs the Issuer ID and Key ID from step 2 on.
  pub(crate) fn go_to_step(&mut self, step: usize, cx: &mut Context<Self>) {
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

  /// The content of a step.
  fn step_content(&self, step: usize, cx: &mut Context<Self>) -> Div {
    match step {
      0 => self.step_create_key(cx),
      1 => self.step_ids(cx),
      _ => self.step_private_key(cx),
    }
  }

  /// A row with a field on the left and a drawing of where to find its value on the right.
  fn field_row(
    l: Language,
    id: &'static str,
    index: u64,
    label: SharedString,
    input: &Entity<InputState>,
    hint: SharedString,
    drawing: Div,
    cx: &App,
  ) -> impl IntoElement {
    let row = h_flex()
      .w_full()
      .gap_6()
      .items_center()
      .py_3()
      .child(
        v_flex()
          .flex_1()
          .min_w_0()
          .gap_1()
          .child(Label::new(label))
          .child(Input::new(input))
          .child(div().text_xs().text_color(cx.theme().muted_foreground).child(hint)),
      )
      .child(
        v_flex()
          .w(px(DRAWING_WIDTH))
          .flex_shrink_0()
          .gap_1p5()
          .child(div().text_xs().text_color(cx.theme().muted_foreground).child(tr(l, "How Apple shows it")))
          .child(drawing),
      );
    reveal_after(id, index * 120, row)
  }

  /// The five things to do on Apple's site, each with a checkbox to tick when it is done.
  fn step_create_key(&self, cx: &mut Context<Self>) -> Div {
    let l = self.settings.language;
    let open_page = div().child(
      Button::new("create-key")
        .icon(Icon::new(IconName::ExternalLink))
        .label(tr(l, "Open App Store Connect"))
        .primary()
        .on_click(|_, _, cx| cx.open_url(API_KEYS_URL)),
    );
    let rows: [(&'static str, SharedString, SharedString, Div); KEY_STEP_COUNT] = [
      (
        "done-open-page",
        tr(l, "Open the key page"),
        tr(l, "Opens in your browser. If it asks you to sign in, use your Apple ID. The path is Users and Access, Integrations, Team Keys."),
        open_page,
      ),
      (
        "done-generate-key",
        tr(l, "Generate a key"),
        tr(l, "Click Generate API Key. If you already have keys, click the + next to Active instead."),
        guide_art::generate_key_drawing(l, cx),
      ),
      (
        "done-name-key",
        tr(l, "Name it and choose a role"),
        tr(l, "Name it \"AppForge\", open Select Roles, choose App Manager and click Generate."),
        guide_art::name_window_drawing(l, &self.login.apple_name, cx),
      ),
      ("done-download-key", tr(l, "Download the key"), tr(l, "Click Download next to the new key."), guide_art::download_drawing(cx)),
      (
        "done-confirm-download",
        tr(l, "Confirm the download"),
        tr(l, "Apple warns that a key can be downloaded only once. Click Download and keep the file in a safe place."),
        guide_art::confirm_drawing(cx),
      ),
    ];
    let mut list = v_flex().w_full();
    for (index, (id, title, text, drawing)) in rows.into_iter().enumerate() {
      // The first drawing is AppForge's own button; the others are pictures of Apple's page, not buttons to press here.
      let caption = if index == 0 { tr(l, "Click here") } else { tr(l, "How Apple shows it") };
      let ticked = self.login.done[index];
      let checkbox = Checkbox::new(id).checked(ticked).tooltip(tr(l, "Mark as done")).on_change(cx.listener(move |this, checked: &bool, _, cx| {
        this.login.done[index] = *checked;
        cx.notify();
      }));
      if index > 0 {
        list = list.child(Separator::horizontal());
      }
      list = list.child(guide_art::step_row(index, checkbox, ticked, title, text, caption, drawing, cx));
    }
    list
  }

  fn step_ids(&self, cx: &mut Context<Self>) -> Div {
    let l = self.settings.language;
    v_flex()
      .w_full()
      .child(Self::field_row(
        l,
        "ids-issuer",
        0,
        tr(l, "Issuer ID"),
        &self.login.issuer,
        tr(l, "A long code with dashes, shown at the top of the key list, above the keys."),
        guide_art::issuer_drawing(cx),
        cx,
      ))
      .child(Separator::horizontal())
      .child(Self::field_row(
        l,
        "ids-key",
        1,
        tr(l, "Key ID"),
        &self.login.key_id,
        tr(l, "A short code of 10 letters and digits, in the \"Key ID\" column of your AppForge key."),
        guide_art::key_id_drawing(cx),
        cx,
      ))
  }

  fn step_private_key(&self, cx: &mut Context<Self>) -> Div {
    let l = self.settings.language;
    let theme = cx.theme().clone();
    let (key_label, chosen): (SharedString, bool) = match &self.login.key {
      Some(key) => (key.file_name.clone().into(), true),
      None => (tr(l, "No file chosen"), false),
    };
    let drawing = guide_art::key_file_drawing(self.login.key.as_ref().map(|key| key.file_name.as_str()), l);
    v_flex()
      .w_full()
      .gap_3()
      .py_3()
      .child(drawing)
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
      )))
  }

  pub(crate) fn render_login(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
    let theme = cx.theme().clone();
    let l = self.settings.language;
    let step = self.login.step.min(STEP_COUNT - 1);
    let body = self.step_content(step, cx);
    let (title, subtitle) = match step {
      0 => (
        tr(l, "Create an API key on Apple's website"),
        Some(tr(l, "AppForge talks to App Store Connect with a key that you create once. It takes about two minutes.")),
      ),
      1 => (tr(l, "Copy two values from the same page"), None),
      _ => (tr(l, "Add the key file you downloaded"), None),
    };

    let done = self.login.done.iter().filter(|done| **done).count();
    let back = (step > 0)
      .then(|| Button::new("login-back").label(tr(l, "Back")).ghost().on_click(cx.listener(move |this, _, _, cx| this.go_to_step(step - 1, cx))));
    // On the first step the button stays quiet until every sub-step is ticked, but it never blocks: a key may exist already.
    let forward = if step + 1 < STEP_COUNT {
      let label = if step == 0 { tr(l, "I have the key file") } else { tr(l, "Continue") };
      let button = Button::new("login-next").label(label).on_click(cx.listener(move |this, _, _, cx| this.go_to_step(step + 1, cx)));
      if step == 0 && done < KEY_STEP_COUNT {
        button.outline()
      } else {
        button.primary()
      }
    } else {
      Button::new("sign-in")
        .label(tr(l, "Sign in"))
        .primary()
        .loading(self.login.busy)
        .disabled(self.login.busy || self.settings_locked)
        .on_click(cx.listener(|this, _, window, cx| this.sign_in(window, cx)))
    };
    let status = (step == 0).then(|| {
      h_flex()
        .gap_3()
        .items_center()
        .child(Tag::secondary().child(trf(l, "{} of {} done", &[&done.to_string(), &KEY_STEP_COUNT.to_string()])))
        .child(div().text_sm().text_color(theme.muted_foreground).child(tr(l, "Tick each step as you finish it on Apple's site.")))
    });
    let nav = h_flex().w_full().gap_2().items_center().justify_between().child(h_flex().gap_2().children(back).children(status)).child(forward);

    let mut content = v_flex()
      .w_full()
      .max_w(px(SETUP_CONTENT_WIDTH))
      .px_10()
      .pt_2()
      .pb_8()
      .gap_6()
      .child(
        v_flex()
          .gap_1()
          .child(div().text_3xl().font_weight(FontWeight::SEMIBOLD).child(title))
          .children(subtitle.map(|subtitle| div().text_color(theme.muted_foreground).child(subtitle))),
      )
      .child(GroupBox::new().outline().child(body));
    if step == 0 {
      content = content.child(
        Alert::info(
          "key-access-note",
          tr(l, "Can't see Integrations or Generate API Key? Only the Account Holder or an Admin can create keys. Ask them to give you access."),
        )
        .title(tr(l, "Need access?")),
      );
    }
    if let Some(error) = &self.login.error {
      content = content.child(Alert::error("login-error", error.clone()));
    }
    self.render_setup(step + 1, content, Some(nav), cx)
  }
}
