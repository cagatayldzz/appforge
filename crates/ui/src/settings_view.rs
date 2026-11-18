use crate::{
  i18n::{tr, trf},
  theme::FONT_SIZE_SM,
  widgets::{centered, setting_row},
  workspace::{SettingsPage, Workspace},
};
use appforge_ai::Provider;
use appforge_compliance::PLAYBOOK_REPOSITORY;
use gpui_kit::{
  component::{
    button::{Button, ButtonVariants as _},
    h_flex,
    input::Input,
    scroll::ScrollableElement as _,
    select::Select,
    separator::Separator,
    tag::Tag,
    v_flex, ActiveTheme, Icon, IconName, Sizable as _,
  },
  *,
};

/// How wide the fields in Settings get.
const FIELD_WIDTH: f32 = 280.;

impl Workspace {
  /// One AI provider: its name with whether it is in use, then its model and key.
  fn provider_section(&mut self, provider: Provider, cx: &mut Context<Self>) -> impl IntoElement {
    let l = self.settings.language;
    let active = self.settings.ai.provider == provider;
    let has_key = self.has_key.get(&provider).copied().unwrap_or(false);
    let (key_input, model_input, key_name) = match provider {
      Provider::OpenAi => (&self.settings_form.openai_key, &self.settings_form.openai_model, "OpenAI API key"),
      Provider::Anthropic => (&self.settings_form.anthropic_key, &self.settings_form.anthropic_model, "Anthropic API key"),
    };
    let status: SharedString =
      if has_key { trf(l, "{} saved in the Keychain. Type a new one to replace it.", &[key_name]) } else { trf(l, "No {} yet.", &[key_name]) };
    let state = if active {
      Tag::success().small().child(tr(l, "In use")).into_any_element()
    } else {
      Button::new(SharedString::from(format!("use-{}", provider.label())))
        .label(trf(l, "Use {}", &[provider.label()]))
        .outline()
        .small()
        .on_click(cx.listener(move |this, _, window, cx| this.set_provider(provider, window, cx)))
        .into_any_element()
    };
    let mut key_actions = h_flex().gap_2().items_center().child(div().w(px(FIELD_WIDTH)).child(Input::new(key_input))).child(
      Button::new(SharedString::from(format!("save-{}", provider.label())))
        .label(tr(l, "Save"))
        .primary()
        .on_click(cx.listener(move |this, _, window, cx| this.save_ai(provider, window, cx))),
    );
    if has_key {
      key_actions = key_actions.child(
        Button::new(SharedString::from(format!("remove-{}", provider.label())))
          .label(tr(l, "Remove key"))
          .ghost()
          .on_click(cx.listener(move |this, _, window, cx| this.remove_key(provider, window, cx))),
      );
    }
    v_flex()
      .w_full()
      .gap_1()
      .child(h_flex().w_full().justify_between().items_center().child(div().font_weight(FontWeight::SEMIBOLD).child(provider.label())).child(state))
      .child(Separator::horizontal())
      .child(setting_row(tr(l, "Model"), None, div().w(px(FIELD_WIDTH)).child(Input::new(model_input)), cx))
      .child(setting_row(tr(l, "API key"), Some(status), key_actions, cx))
  }

  fn settings_page_body(&mut self, cx: &mut Context<Self>) -> AnyElement {
    let l = self.settings.language;
    match self.settings_page {
      SettingsPage::General => v_flex()
        .child(setting_row(
          tr(l, "Language"),
          Some(tr(l, "Choose the language of AppForge. The App Store listing you edit keeps its own languages.")),
          div().w(px(FIELD_WIDTH)).child(Select::new(&self.language_select)),
          cx,
        ))
        .into_any_element(),
      SettingsPage::Account => {
        let account = match &self.settings.apple {
          Some(account) => trf(l, "Signed in with key {}.", &[&account.key_id]),
          None => tr(l, "Not signed in."),
        };
        v_flex()
          .child(setting_row(
            tr(l, "Account"),
            Some(account),
            Button::new("settings-sign-out").label(tr(l, "Sign out")).outline().on_click(cx.listener(|this, _, window, cx| this.sign_out(window, cx))),
            cx,
          ))
          .into_any_element()
      }
      SettingsPage::AiProvider => {
        let sections: Vec<_> = Provider::ALL.iter().map(|provider| self.provider_section(*provider, cx).into_any_element()).collect();
        v_flex()
          .gap_6()
          .children(sections)
          .child(div().text_size(FONT_SIZE_SM).text_color(cx.theme().muted_foreground).child(tr(l, "More providers can be added later.")))
          .into_any_element()
      }
      SettingsPage::Rules => v_flex()
        .child(setting_row(
          tr(l, "Rules"),
          Some(tr(l, "The Analysis reads the App Store Compliance Playbook. AppForge downloads it from GitHub and refreshes it weekly; you can also update it from the Analysis page.")),
          Button::new("settings-playbook").label(tr(l, "Open the playbook on GitHub")).outline().on_click(|_, _, cx| cx.open_url(PLAYBOOK_REPOSITORY)),
          cx,
        ))
        .into_any_element(),
      SettingsPage::About => {
        let data_dir = self.data_dir.clone();
        v_flex()
          .child(setting_row(
            trf(l, "AppForge version {}", &[env!("CARGO_PKG_VERSION")]),
            Some(tr(l, "Settings and notes are stored in the data folder. API keys are in the Keychain, never in a file.")),
            Button::new("settings-data-folder")
              .icon(Icon::new(IconName::FolderOpen))
              .label(tr(l, "Show data folder"))
              .outline()
              .on_click(move |_, _, cx| cx.reveal_path(&data_dir)),
            cx,
          ))
          .into_any_element()
      }
    }
  }

  pub(crate) fn render_settings(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
    let l = self.settings.language;
    let muted = cx.theme().muted_foreground;
    let heading = v_flex().gap_1().child(div().text_xl().font_weight(FontWeight::SEMIBOLD).child(tr(l, self.settings_page.label()))).children(
      (self.settings_page == SettingsPage::AiProvider).then(|| {
        div()
          .text_size(FONT_SIZE_SM)
          .text_color(muted)
          .child(tr(l, "AI analysis uses your own API key. Keys are kept in this Mac's Keychain, never in a file."))
      }),
    );
    let body = self.settings_page_body(cx);
    v_flex().id("settings").flex_1().min_h_0().overflow_y_scrollbar().child(centered(v_flex().py_6().gap_4().child(heading).child(body)))
  }
}
