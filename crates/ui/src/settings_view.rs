use crate::{
  i18n::{tr, trf},
  theme::{colors, FONT_SIZE_SM},
  widgets::{app_avatar, centered, setting_row},
  workspace::{Load, SettingsPage, Workspace},
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
    v_flex, ActiveTheme, Icon, IconName, Sizable as _,
  },
  *,
};

/// How wide the fields in Settings get.
const FIELD_WIDTH: f32 = 360.;

impl Workspace {
  /// One AI provider: its name with whether it is in use, then its model and key, each under its label.
  fn provider_section(&mut self, provider: Provider, cx: &mut Context<Self>) -> impl IntoElement {
    let c = colors(cx);
    let l = self.settings.language;
    let active = self.settings.ai.provider == provider;
    let has_key = self.has_key.get(&provider).copied().unwrap_or(false);
    let (key_input, model_select, key_name) = match provider {
      Provider::OpenAi => (&self.settings_form.openai_key, &self.settings_form.openai_model, "OpenAI API key"),
      Provider::Anthropic => (&self.settings_form.anthropic_key, &self.settings_form.anthropic_model, "Anthropic API key"),
    };
    let status: SharedString =
      if has_key { trf(l, "{} saved in the Keychain. Type a new one to replace it.", &[key_name]) } else { trf(l, "No {} yet.", &[key_name]) };
    // The provider that is in use shows a dot and its state; the others offer to become the one in use.
    let price_hint: SharedString = match provider.model_info(self.settings.ai.model(provider)) {
      Some(info) => trf(
        l,
        "Price per million tokens: {} in, {} out. Only low-cost models are offered. Prices can change, so check the provider's price page.",
        &[&format!("${:.2}", info.input_per_million), &format!("${:.2}", info.output_per_million)],
      ),
      None => SharedString::default(),
    };
    let state = if active {
      h_flex().gap_1p5().items_center().child(div().size_2().rounded_full().bg(c.success)).child(tr(l, "In use")).into_any_element()
    } else {
      Button::new(SharedString::from(format!("use-{}", provider.label())))
        .label(trf(l, "Use {}", &[provider.label()]))
        .outline()
        .small()
        .on_click(cx.listener(move |this, _, window, cx| this.set_provider(provider, window, cx)))
        .into_any_element()
    };
    let mut key_row = h_flex().gap_2().items_center().child(div().w(px(FIELD_WIDTH)).child(Input::new(key_input))).child(
      Button::new(SharedString::from(format!("save-{}", provider.label())))
        .label(tr(l, "Save"))
        .primary()
        .on_click(cx.listener(move |this, _, window, cx| this.save_ai(provider, window, cx))),
    );
    if has_key {
      key_row = key_row.child(
        Button::new(SharedString::from(format!("remove-{}", provider.label())))
          .label(tr(l, "Remove key"))
          .ghost()
          .on_click(cx.listener(move |this, _, window, cx| this.remove_key(provider, window, cx))),
      );
    }
    let field = |label: SharedString, control: AnyElement, hint: Option<SharedString>| {
      v_flex()
        .gap_1p5()
        .child(div().font_weight(FontWeight::MEDIUM).child(label))
        .child(control)
        .children(hint.map(|hint| div().text_size(FONT_SIZE_SM).text_color(c.text_secondary).child(hint)))
    };
    v_flex()
      .w_full()
      .gap_3()
      .child(
        h_flex().w_full().h_8().justify_between().items_center().child(div().font_weight(FontWeight::SEMIBOLD).child(provider.label())).child(state),
      )
      .child(Separator::horizontal())
      .child(
        v_flex()
          .gap_4()
          .pt_1()
          .child(field(tr(l, "Model"), div().w(px(FIELD_WIDTH)).child(Select::new(model_select)).into_any_element(), Some(price_hint)))
          .child(field(tr(l, "API key"), key_row.into_any_element(), Some(status))),
      )
  }

  /// One row per app: where its source code is, with a button to choose or remove the folder.
  fn code_folders_body(&mut self, cx: &mut Context<Self>) -> AnyElement {
    let c = colors(cx);
    let l = self.settings.language;
    let apps = match &self.apps {
      Load::Ready(apps) => apps.clone(),
      _ => Vec::new(),
    };
    let intro = div()
      .text_size(FONT_SIZE_SM)
      .text_color(c.text_secondary)
      .child(tr(l, "Choose the folder that holds each app's source code, so AppForge can read the project."));
    if apps.is_empty() {
      return v_flex()
        .gap_3()
        .child(intro)
        .child(div().text_size(FONT_SIZE_SM).text_color(c.text_secondary).child(tr(l, "Loading apps…")))
        .into_any_element();
    }
    let rows: Vec<AnyElement> = apps
      .into_iter()
      .map(|app| {
        let folder = self.settings.code_folders.get(&app.id).cloned();
        let missing = folder.as_ref().is_some_and(|folder| !folder.is_dir());
        let shown: SharedString = match &folder {
          Some(folder) => folder.display().to_string().into(),
          None => tr(l, "Not set"),
        };
        let (choose_id, clear_id) = (app.id.clone(), app.id.clone());
        let mut actions = h_flex().gap_2().child(
          Button::new(SharedString::from(format!("choose-folder-{}", app.id)))
            .icon(Icon::new(IconName::FolderOpen))
            .label(if folder.is_some() { tr(l, "Change") } else { tr(l, "Choose folder") })
            .outline()
            .small()
            .on_click(cx.listener(move |this, _, window, cx| this.choose_code_folder(choose_id.clone(), false, window, cx))),
        );
        if folder.is_some() {
          actions = actions.child(
            Button::new(SharedString::from(format!("clear-folder-{}", app.id)))
              .label(tr(l, "Remove"))
              .ghost()
              .small()
              .on_click(cx.listener(move |this, _, window, cx| this.clear_code_folder(&clear_id, window, cx))),
          );
        }
        let path_color = if missing {
          c.danger
        } else if folder.is_some() {
          c.text
        } else {
          c.text_secondary
        };
        h_flex()
          .w_full()
          .gap_4()
          .py_3()
          .items_center()
          .border_b_1()
          .border_color(c.border)
          .child(app_avatar(&app.id, &app.name, true, false, self.icons.get(&app.id), cx).size_8())
          .child(
            v_flex()
              .flex_1()
              .min_w_0()
              .gap_0p5()
              .child(div().font_weight(FontWeight::MEDIUM).child(app.name.clone()))
              .child(div().truncate().text_size(FONT_SIZE_SM).text_color(path_color).child(shown))
              .children(missing.then(|| div().text_size(FONT_SIZE_SM).text_color(c.danger).child(tr(l, "This folder was not found.")))),
          )
          .child(actions)
          .into_any_element()
      })
      .collect();
    v_flex().gap_2().child(intro).child(v_flex().children(rows)).into_any_element()
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
          .gap_8()
          .children(sections)
          .child(div().text_size(FONT_SIZE_SM).text_color(cx.theme().muted_foreground).child(tr(l, "More providers can be added later.")))
          .into_any_element()
      }
      SettingsPage::CodeFolders => self.code_folders_body(cx),
      SettingsPage::Rules => v_flex()
        .gap_4()
        .child(self.render_guidelines(cx))
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
