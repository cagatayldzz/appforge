use crate::{
  i18n::{tr, trf},
  widgets::{page_title, CONTENT_MAX_WIDTH},
  workspace::Workspace,
};
use appforge_ai::Provider;
use appforge_compliance::PLAYBOOK_REPOSITORY;
use gpui_kit::{
  component::{
    button::{Button, ButtonVariants as _},
    h_flex,
    input::Input,
    label::Label,
    tag::Tag,
    v_flex, ActiveTheme, Disableable as _, Icon, IconName, Sizable as _,
  },
  *,
};

impl Workspace {
  fn provider_card(&mut self, provider: Provider, cx: &mut Context<Self>) -> impl IntoElement {
    let theme = cx.theme().clone();
    let l = self.settings.language;
    let active = self.settings.ai.provider == provider;
    let has_key = self.has_key.get(&provider).copied().unwrap_or(false);
    let (key_input, model_input, key_placeholder) = match provider {
      Provider::OpenAi => (&self.settings_form.openai_key, &self.settings_form.openai_model, "OpenAI API key"),
      Provider::Anthropic => (&self.settings_form.anthropic_key, &self.settings_form.anthropic_model, "Anthropic API key"),
    };
    let status: SharedString = if has_key {
      trf(l, "{} saved in the Keychain. Type a new one to replace it.", &[key_placeholder])
    } else {
      trf(l, "No {} yet.", &[key_placeholder])
    };
    let tag_or_button = if active {
      Tag::success().small().child(tr(l, "In use")).into_any_element()
    } else {
      Button::new(SharedString::from(format!("use-{}", provider.label())))
        .label(trf(l, "Use {}", &[provider.label()]))
        .outline()
        .small()
        .on_click(cx.listener(move |this, _, window, cx| this.set_provider(provider, window, cx)))
        .into_any_element()
    };
    let mut actions = h_flex().gap_2().child(
      Button::new(SharedString::from(format!("save-{}", provider.label())))
        .label(tr(l, "Save"))
        .primary()
        .on_click(cx.listener(move |this, _, window, cx| this.save_ai(provider, window, cx))),
    );
    if has_key {
      actions = actions.child(
        Button::new(SharedString::from(format!("remove-{}", provider.label())))
          .label(tr(l, "Remove key"))
          .ghost()
          .on_click(cx.listener(move |this, _, window, cx| this.remove_key(provider, window, cx))),
      );
    }
    v_flex()
      .gap_3()
      .p_4()
      .rounded_md()
      .border_1()
      .border_color(theme.border)
      .child(h_flex().justify_between().items_center().child(div().font_weight(FontWeight::SEMIBOLD).child(provider.label())).child(tag_or_button))
      .child(v_flex().gap_1().child(Label::new(tr(l, "Model"))).child(Input::new(model_input)))
      .child(
        v_flex()
          .gap_1()
          .child(Label::new(tr(l, "API key")))
          .child(Input::new(key_input))
          .child(div().text_xs().text_color(theme.muted_foreground).child(status)),
      )
      .child(actions)
  }

  pub(crate) fn render_settings(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
    let theme = cx.theme().clone();
    let l = self.settings.language;
    let heading = |text: SharedString| div().pt_4().text_lg().font_weight(FontWeight::SEMIBOLD).child(text);
    let cards: Vec<_> = Provider::ALL.iter().map(|provider| self.provider_card(*provider, cx).into_any_element()).collect();
    let picker = self.language_picker("settings-language", false, cx);
    let account = match &self.settings.apple {
      Some(account) => trf(l, "Signed in with key {}.", &[&account.key_id]),
      None => tr(l, "Not signed in."),
    };
    let data_dir = self.data_dir.clone();
    v_flex()
      .w_full()
      .max_w(px(CONTENT_MAX_WIDTH))
      .px_8()
      .pt_4()
      .pb_8()
      .gap_4()
      .child(page_title(tr(l, "Settings"), tr(l, "AI analysis uses your own API key. Keys are kept in this Mac's Keychain, never in a file."), cx))
      .child(heading(tr(l, "General")))
      .child(
        v_flex()
          .gap_2()
          .child(Label::new(tr(l, "Language")))
          .child(picker)
          .child(div().text_xs().text_color(theme.muted_foreground).child(tr(l, "Choose the language of AppForge. The App Store listing you edit keeps its own languages."))),
      )
      .child(heading(tr(l, "Account")))
      .child(
        h_flex()
          .gap_3()
          .items_center()
          .child(div().text_sm().text_color(theme.muted_foreground).child(account))
          .child(Button::new("settings-sign-out").label(tr(l, "Sign out")).outline().small().on_click(cx.listener(|this, _, window, cx| this.sign_out(window, cx)))),
      )
      .child(heading(tr(l, "AI provider")))
      .children(cards)
      .child(div().text_sm().text_color(theme.muted_foreground).child(tr(l, "More providers can be added later.")))
      .child(heading(tr(l, "Rules")))
      .child(div().text_sm().text_color(theme.muted_foreground).child(tr(l, "The Analysis reads the App Store Compliance Playbook. AppForge downloads it from GitHub and refreshes it weekly; you can also update it from the Analysis page.")))
      .child(Button::new("settings-playbook").label(tr(l, "Open the playbook on GitHub")).outline().disabled(false).on_click(|_, _, cx| cx.open_url(PLAYBOOK_REPOSITORY)))
      .child(heading(tr(l, "About")))
      .child(div().text_sm().child(trf(l, "AppForge version {}", &[env!("CARGO_PKG_VERSION")])))
      .child(div().text_sm().text_color(theme.muted_foreground).child(tr(l, "Settings and notes are stored in the data folder. API keys are in the Keychain, never in a file.")))
      .child(
        h_flex().child(
          Button::new("settings-data-folder")
            .icon(Icon::new(IconName::FolderOpen))
            .label(tr(l, "Show data folder"))
            .outline()
            .on_click(move |_, _, cx| cx.reveal_path(&data_dir)),
        ),
      )
  }
}
