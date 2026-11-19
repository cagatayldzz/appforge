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
      self.login.error = Some(tr(l, "enterBothTheIssuer"));
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
          .child(div().text_xs().text_color(cx.theme().muted_foreground).child(tr(l, "howAppleShowsIt")))
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
        .label(tr(l, "openAppStoreConnect"))
        .primary()
        .on_click(|_, _, cx| cx.open_url(API_KEYS_URL)),
    );
    let rows: [(&'static str, SharedString, SharedString, Div); KEY_STEP_COUNT] = [
      ("done-open-page", tr(l, "openTheKeyPage"), tr(l, "opensInYourBrowser"), open_page),
      ("done-generate-key", tr(l, "generateAKey"), tr(l, "clickGenerateApiKey"), guide_art::generate_key_drawing(l, cx)),
      ("done-name-key", tr(l, "nameItAndChoose"), tr(l, "nameItAppforgeOpen"), guide_art::name_window_drawing(l, &self.login.apple_name, cx)),
      ("done-download-key", tr(l, "downloadTheKey"), tr(l, "clickDownloadNextTo"), guide_art::download_drawing(cx)),
      ("done-confirm-download", tr(l, "confirmTheDownload"), tr(l, "appleWarnsThatA"), guide_art::confirm_drawing(cx)),
    ];
    let mut list = v_flex().w_full();
    for (index, (id, title, text, drawing)) in rows.into_iter().enumerate() {
      // The first drawing is AppForge's own button; the others are pictures of Apple's page, not buttons to press here.
      let caption = if index == 0 { tr(l, "clickHere") } else { tr(l, "howAppleShowsIt") };
      let ticked = self.login.done[index];
      let checkbox = Checkbox::new(id).checked(ticked).tooltip(tr(l, "markAsDone")).on_change(cx.listener(move |this, checked: &bool, _, cx| {
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
      .child(Self::field_row(l, "ids-issuer", 0, tr(l, "issuerId"), &self.login.issuer, tr(l, "aLongCodeWith"), guide_art::issuer_drawing(cx), cx))
      .child(Separator::horizontal())
      .child(Self::field_row(l, "ids-key", 1, tr(l, "keyId"), &self.login.key_id, tr(l, "aShortCodeOf"), guide_art::key_id_drawing(cx), cx))
  }

  fn step_private_key(&self, cx: &mut Context<Self>) -> Div {
    let l = self.settings.language;
    let theme = cx.theme().clone();
    let (key_label, chosen): (SharedString, bool) = match &self.login.key {
      Some(key) => (key.file_name.clone().into(), true),
      None => (tr(l, "noFileChosen"), false),
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
              .label(if chosen { tr(l, "chooseAnotherFile") } else { tr(l, "chooseP8File") })
              .outline()
              .on_click(cx.listener(|this, _, window, cx| this.choose_key_file(window, cx))),
          )
          .child(div().text_sm().text_color(theme.muted_foreground).child(key_label)),
      )
      .child(div().text_xs().text_color(theme.muted_foreground).child(tr(l, "theFileIsNamed")))
  }

  pub(crate) fn render_login(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
    let theme = cx.theme().clone();
    let l = self.settings.language;
    let step = self.login.step.min(STEP_COUNT - 1);
    let body = self.step_content(step, cx);
    let (title, subtitle) = match step {
      0 => (tr(l, "createAnApiKey"), Some(tr(l, "appforgeTalksToApp"))),
      1 => (tr(l, "copyTwoValuesFrom"), None),
      _ => (tr(l, "addTheKeyFile"), None),
    };

    let done = self.login.done.iter().filter(|done| **done).count();
    let back = (step > 0)
      .then(|| Button::new("login-back").label(tr(l, "back")).ghost().on_click(cx.listener(move |this, _, _, cx| this.go_to_step(step - 1, cx))));
    // On the first step the button stays quiet until every sub-step is ticked, but it never blocks: a key may exist already.
    let forward = if step + 1 < STEP_COUNT {
      let label = if step == 0 { tr(l, "iHaveTheKey") } else { tr(l, "continue") };
      let button = Button::new("login-next").label(label).on_click(cx.listener(move |this, _, _, cx| this.go_to_step(step + 1, cx)));
      if step == 0 && done < KEY_STEP_COUNT {
        button.outline()
      } else {
        button.primary()
      }
    } else {
      Button::new("sign-in")
        .label(tr(l, "signIn"))
        .primary()
        .loading(self.login.busy)
        .disabled(self.login.busy || self.settings_locked)
        .on_click(cx.listener(|this, _, window, cx| this.sign_in(window, cx)))
    };
    let status = (step == 0).then(|| {
      h_flex()
        .gap_3()
        .items_center()
        .child(Tag::secondary().child(trf(l, "ofDone", &[&done.to_string(), &KEY_STEP_COUNT.to_string()])))
        .child(div().text_sm().text_color(theme.muted_foreground).child(tr(l, "tickEachStepAs")))
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
      content = content.child(Alert::info("key-access-note", tr(l, "cantSeeIntegrationsOr")).title(tr(l, "needAccess")));
    }
    if let Some(error) = &self.login.error {
      content = content.child(Alert::error("login-error", error.clone()));
    }
    self.render_setup(step + 1, content, Some(nav), cx)
  }
}
