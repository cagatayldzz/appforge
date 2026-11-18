use crate::{
  i18n::{tr, trf},
  widgets::{centered, page_title, section},
  workspace::Workspace,
};
use appforge_ai::Provider;
use appforge_compliance::PLAYBOOK_REPOSITORY;
use gpui_kit::{
  component::{
    button::{Button, ButtonVariants as _},
    group_box::{GroupBox, GroupBoxVariants as _},
    h_flex,
    input::Input,
    label::Label,
    scroll::ScrollableElement as _,
    select::Select,
    tag::Tag,
    v_flex, ActiveTheme, Disableable as _, Icon, IconName, Sizable as _,
  },
  *,
};

/// How wide the language menu in Settings gets.
const LANGUAGE_MENU_WIDTH: f32 = 240.;

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
    GroupBox::new()
      .outline()
      .title(
        h_flex()
          .w_full()
          .justify_between()
          .items_center()
          .child(div().font_weight(FontWeight::SEMIBOLD).child(provider.label()))
          .child(tag_or_button),
      )
      .child(
        v_flex()
          .w_full()
          .gap_3()
          .child(v_flex().gap_1().child(Label::new(tr(l, "Model"))).child(Input::new(model_input)))
          .child(
            v_flex()
              .gap_1()
              .child(Label::new(tr(l, "API key")))
              .child(Input::new(key_input))
              .child(div().text_xs().text_color(theme.muted_foreground).child(status)),
          )
          .child(actions),
      )
  }

  pub(crate) fn render_settings(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
    let theme = cx.theme().clone();
    let l = self.settings.language;
    let cards: Vec<_> = Provider::ALL.iter().map(|provider| self.provider_card(*provider, cx).into_any_element()).collect();
    let account = match &self.settings.apple {
      Some(account) => trf(l, "Signed in with key {}.", &[&account.key_id]),
      None => tr(l, "Not signed in."),
    };
    let data_dir = self.data_dir.clone();
    let muted = |text: SharedString| div().text_sm().text_color(theme.muted_foreground).child(text);
    let page = v_flex()
      .gap_6()
      .py_6()
      .child(page_title(tr(l, "Settings"), tr(l, "AI analysis uses your own API key. Keys are kept in this Mac's Keychain, never in a file."), cx))
      .child(section(
        tr(l, "General"),
        v_flex()
          .gap_2()
          .child(Label::new(tr(l, "Language")))
          .child(div().w_full().max_w(px(LANGUAGE_MENU_WIDTH)).child(Select::new(&self.language_select)))
          .child(div().text_xs().text_color(theme.muted_foreground).child(tr(l, "Choose the language of AppForge. The App Store listing you edit keeps its own languages."))),
      ))
      .child(section(
        tr(l, "Account"),
        h_flex()
          .gap_3()
          .items_center()
          .justify_between()
          .child(muted(account))
          .child(Button::new("settings-sign-out").label(tr(l, "Sign out")).outline().small().on_click(cx.listener(|this, _, window, cx| this.sign_out(window, cx)))),
      ))
      .child(
        v_flex()
          .gap_3()
          .child(div().text_lg().font_weight(FontWeight::SEMIBOLD).child(tr(l, "AI provider")))
          .children(cards)
          .child(muted(tr(l, "More providers can be added later."))),
      )
      .child(section(
        tr(l, "Rules"),
        v_flex()
          .gap_3()
          .child(muted(tr(l, "The Analysis reads the App Store Compliance Playbook. AppForge downloads it from GitHub and refreshes it weekly; you can also update it from the Analysis page.")))
          .child(h_flex().child(Button::new("settings-playbook").label(tr(l, "Open the playbook on GitHub")).outline().disabled(false).on_click(|_, _, cx| cx.open_url(PLAYBOOK_REPOSITORY)))),
      ))
      .child(section(
        tr(l, "About"),
        v_flex()
          .gap_3()
          .child(div().text_sm().child(trf(l, "AppForge version {}", &[env!("CARGO_PKG_VERSION")])))
          .child(muted(tr(l, "Settings and notes are stored in the data folder. API keys are in the Keychain, never in a file.")))
          .child(h_flex().child(
            Button::new("settings-data-folder")
              .icon(Icon::new(IconName::FolderOpen))
              .label(tr(l, "Show data folder"))
              .outline()
              .on_click(move |_, _, cx| cx.reveal_path(&data_dir)),
          )),
      ));
    v_flex().id("settings").flex_1().min_h_0().overflow_y_scrollbar().child(centered(page))
  }
}
