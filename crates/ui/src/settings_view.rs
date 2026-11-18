use crate::{
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
    v_flex, ActiveTheme, Disableable as _, Sizable as _,
  },
  *,
};

impl Workspace {
  fn provider_card(&mut self, provider: Provider, cx: &mut Context<Self>) -> impl IntoElement {
    let theme = cx.theme().clone();
    let active = self.settings.ai.provider == provider;
    let has_key = self.has_key.get(&provider).copied().unwrap_or(false);
    let (key_input, model_input, key_placeholder) = match provider {
      Provider::OpenAi => (&self.settings_form.openai_key, &self.settings_form.openai_model, "OpenAI API key"),
      Provider::Anthropic => (&self.settings_form.anthropic_key, &self.settings_form.anthropic_model, "Anthropic API key"),
    };
    let status: SharedString = if has_key {
      format!("{key_placeholder} saved in the Keychain. Type a new one to replace it.").into()
    } else {
      format!("No {key_placeholder} yet.").into()
    };
    let tag_or_button = if active {
      Tag::success().small().child("In use").into_any_element()
    } else {
      Button::new(SharedString::from(format!("use-{}", provider.label())))
        .label(format!("Use {}", provider.label()))
        .outline()
        .small()
        .on_click(cx.listener(move |this, _, window, cx| this.set_provider(provider, window, cx)))
        .into_any_element()
    };
    let mut actions = h_flex().gap_2().child(
      Button::new(SharedString::from(format!("save-{}", provider.label())))
        .label("Save")
        .primary()
        .on_click(cx.listener(move |this, _, window, cx| this.save_ai(provider, window, cx))),
    );
    if has_key {
      actions = actions.child(
        Button::new(SharedString::from(format!("remove-{}", provider.label())))
          .label("Remove key")
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
      .child(v_flex().gap_1().child(Label::new("Model")).child(Input::new(model_input)))
      .child(
        v_flex()
          .gap_1()
          .child(Label::new("API key"))
          .child(Input::new(key_input))
          .child(div().text_xs().text_color(theme.muted_foreground).child(status)),
      )
      .child(actions)
  }

  pub(crate) fn render_settings(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
    let theme = cx.theme().clone();
    let cards: Vec<_> = Provider::ALL.iter().map(|provider| self.provider_card(*provider, cx).into_any_element()).collect();
    v_flex()
      .w_full()
      .max_w(px(CONTENT_MAX_WIDTH))
      .px_8()
      .pt_4()
      .pb_8()
      .gap_4()
      .child(page_title("Settings", "AI analysis uses your own API key. Keys are kept in this Mac's Keychain, never in a file.", cx))
      .child(div().text_lg().font_weight(FontWeight::SEMIBOLD).child("AI provider"))
      .children(cards)
      .child(div().text_sm().text_color(theme.muted_foreground).child("More providers can be added later."))
      .child(div().pt_4().text_lg().font_weight(FontWeight::SEMIBOLD).child("Rules"))
      .child(div().text_sm().text_color(theme.muted_foreground).child("The Analysis reads the App Store Compliance Playbook. AppForge downloads it from GitHub and refreshes it weekly; you can also update it from the Analysis page."))
      .child(Button::new("settings-playbook").label("Open the playbook on GitHub").outline().disabled(false).on_click(|_, _, cx| cx.open_url(PLAYBOOK_REPOSITORY)))
  }
}
