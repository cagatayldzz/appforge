//! The pages of Settings: general, account, AI provider and usage, code folders, rules, about.

use crate::{
  i18n::{tr, trf},
  theme::{colors, FONT_SIZE_SM},
  widgets::{app_avatar, centered_with_width, page_columns, setting_row},
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
    v_flex, ActiveTheme, Disableable as _, Icon, IconName, Sizable as _, WindowExt as _,
  },
  *,
};

/// How wide the fields in Settings get.
const FIELD_WIDTH: f32 = 360.;

/// What stands in the field of a saved key: a fixed mask, so it says nothing about the key itself.
const SAVED_KEY_MASK: &str = "••••••••••••••••••••••••";

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
    let status: SharedString = if has_key { trf(l, "savedInTheKeychain", &[key_name]) } else { trf(l, "noYet", &[key_name]) };
    // The provider that is in use shows a dot and its state; the others offer to become the one in use.
    let price_hint: SharedString = match provider.model_info(self.settings.ai.model(provider)) {
      Some(info) => trf(l, "pricePerMillionTokens", &[&format!("${:.2}", info.input_per_million), &format!("${:.2}", info.output_per_million)]),
      None => SharedString::default(),
    };
    let state = if active {
      h_flex().gap_1p5().items_center().child(div().size_2().rounded_full().bg(c.success)).child(tr(l, "inUse")).into_any_element()
    } else {
      Button::new(SharedString::from(format!("use-{}", provider.label())))
        .label(trf(l, "useProvider", &[provider.label()]))
        .outline()
        .small()
        .on_click(cx.listener(move |this, _, window, cx| this.set_provider(provider, window, cx)))
        .into_any_element()
    };
    let editing = self.editing_key == Some(provider);
    let key_row = if has_key && !editing {
      // A saved key is never read back from the Keychain; the field only shows that one is there.
      h_flex()
        .w_full()
        .gap_2()
        .items_center()
        .child(
          h_flex()
            .id(SharedString::from(format!("saved-key-{}", provider.label())))
            .flex_1()
            .min_w_0()
            .h_8()
            .px_3()
            .gap_2()
            .items_center()
            .justify_between()
            .rounded_md()
            .border_1()
            .border_color(c.border)
            .bg(c.surface_alt)
            .text_color(c.text_secondary)
            .child(div().truncate().child(SAVED_KEY_MASK))
            .child(Icon::new(IconName::Check).small().text_color(c.success)),
        )
        .child(
          Button::new(SharedString::from(format!("replace-{}", provider.label())))
            .label(tr(l, "replaceKey"))
            .outline()
            .on_click(cx.listener(move |this, _, window, cx| this.start_key_edit(provider, window, cx))),
        )
        .child(
          Button::new(SharedString::from(format!("remove-{}", provider.label())))
            .label(tr(l, "removeKey"))
            .ghost()
            .on_click(cx.listener(move |this, _, window, cx| this.remove_key(provider, window, cx))),
        )
    } else {
      h_flex()
        .w_full()
        .gap_2()
        .items_center()
        .child(div().flex_1().min_w_0().child(Input::new(key_input)))
        .child(
          Button::new(SharedString::from(format!("save-{}", provider.label())))
            .label(tr(l, "save"))
            .primary()
            .on_click(cx.listener(move |this, _, window, cx| this.save_ai(provider, window, cx))),
        )
        .children(has_key.then(|| {
          Button::new(SharedString::from(format!("cancel-key-{}", provider.label()))).label(tr(l, "cancel")).ghost().on_click(cx.listener(
            move |this, _, _, cx| {
              this.editing_key = None;
              cx.notify();
            },
          ))
        }))
    };
    let field = |label: SharedString, control: AnyElement, hint: Option<SharedString>| {
      v_flex()
        .w_full()
        .gap_1p5()
        .child(div().text_size(FONT_SIZE_SM).font_weight(FontWeight::MEDIUM).text_color(c.text_secondary).child(label))
        .child(control)
        .children(hint.map(|hint| div().text_size(FONT_SIZE_SM).text_color(c.text_secondary).child(hint)))
    };
    v_flex()
      .w_full()
      .gap_4()
      .p_5()
      .rounded_lg()
      .border_1()
      .border_color(if active { c.success } else { c.border })
      .child(
        h_flex().w_full().h_8().justify_between().items_center().child(div().font_weight(FontWeight::SEMIBOLD).child(provider.label())).child(state),
      )
      .child(field(tr(l, "model"), div().w_full().child(Select::new(model_select)).into_any_element(), Some(price_hint)))
      .child(field(tr(l, "apiKey"), key_row.into_any_element(), Some(status)))
  }

  /// What the AI features have used so far, with a button that clears the totals after a confirmation.
  fn usage_section(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
    let c = colors(cx);
    let l = self.settings.language;
    let total = self.usage.total();
    let since: SharedString =
      if self.usage.since == 0 { tr(l, "nothingWasCountedYet") } else { trf(l, "countedSince", &[&date_text(self.usage.since)]) };
    let tile = |label: SharedString, value: String, detail: Option<String>| {
      v_flex()
        .flex_1()
        .min_w_0()
        .gap_0p5()
        .p_3()
        .rounded_lg()
        .border_1()
        .border_color(c.border)
        .child(div().text_size(FONT_SIZE_SM).text_color(c.text_secondary).child(label))
        .child(div().text_xl().font_weight(FontWeight::SEMIBOLD).child(value))
        .children(detail.map(|detail| div().text_size(FONT_SIZE_SM).text_color(c.text_secondary).child(detail)))
    };
    let tiles = h_flex()
      .w_full()
      .gap_3()
      .child(tile(tr(l, "requests"), total.requests.to_string(), None))
      .child(tile(
        tr(l, "tokens"),
        tokens_text(total.tokens()),
        Some(trf(l, "inOut", &[&tokens_text(total.input_tokens), &tokens_text(total.output_tokens)]).to_string()),
      ))
      .child(tile(tr(l, "estimatedCost"), cost_text(total.cost), None));
    let rows = self.usage.models.iter().map(|entry| {
      let name = entry.provider.model_info(&entry.model).map_or(entry.model.as_str(), |info| info.label);
      h_flex()
        .w_full()
        .gap_3()
        .py_2()
        .border_b_1()
        .border_color(c.border)
        .text_size(FONT_SIZE_SM)
        .child(div().flex_1().min_w_0().truncate().child(format!("{} · {name}", entry.provider.label())))
        .child(div().text_color(c.text_secondary).child(trf(l, "requestsCount", &[&entry.totals.requests.to_string()])))
        .child(div().w_20().text_right().text_color(c.text_secondary).child(tokens_text(entry.totals.tokens())))
        .child(div().w_20().text_right().font_weight(FontWeight::MEDIUM).child(cost_text(entry.totals.cost)))
    });
    let reset = Button::new("reset-ai-usage")
      .icon(Icon::new(IconName::Delete))
      .label(tr(l, "reset"))
      .outline()
      .small()
      .disabled(total.requests == 0)
      .on_click(cx.listener(|this, _, window, cx| this.confirm_reset_usage(window, cx)));
    v_flex()
      .w_full()
      .gap_3()
      .child(
        h_flex()
          .w_full()
          .h_8()
          .justify_between()
          .items_center()
          .child(
            v_flex()
              .child(div().font_weight(FontWeight::SEMIBOLD).child(tr(l, "aiUsage")))
              .child(div().text_size(FONT_SIZE_SM).text_color(c.text_secondary).child(since)),
          )
          .child(reset),
      )
      .child(Separator::horizontal())
      .child(tiles)
      .children(rows)
      .child(div().text_size(FONT_SIZE_SM).text_color(c.text_secondary).child(tr(l, "costsAreEstimatesFrom")))
  }

  /// Asks before the totals are cleared; clearing cannot be undone.
  fn confirm_reset_usage(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let l = self.settings.language;
    let workspace = cx.entity();
    window.open_dialog(cx, move |dialog, _, _| {
      let workspace = workspace.clone();
      dialog.title(tr(l, "resetAiUsage")).child(tr(l, "theCountedRequestsTokens")).footer(
        h_flex()
          .w_full()
          .gap_2()
          .justify_end()
          .child(Button::new("cancel-reset-usage").label(tr(l, "cancel")).outline().on_click(|_, window, cx| window.close_dialog(cx)))
          .child(Button::new("confirm-reset-usage").label(tr(l, "reset")).danger().on_click(move |_, window, cx| {
            window.close_dialog(cx);
            workspace.update(cx, |this, cx| this.reset_usage(window, cx));
          })),
      )
    });
  }

  /// One row per app: where its source code is, with a button to choose or remove the folder.
  fn code_folders_body(&mut self, cx: &mut Context<Self>) -> AnyElement {
    let c = colors(cx);
    let l = self.settings.language;
    let apps = self.app_list();
    let intro = div().text_size(FONT_SIZE_SM).text_color(c.text_secondary).child(tr(l, "chooseTheFolderThat"));
    if apps.is_empty() {
      return v_flex()
        .gap_3()
        .child(intro)
        .child(div().text_size(FONT_SIZE_SM).text_color(c.text_secondary).child(tr(l, "loadingApps")))
        .into_any_element();
    }
    let rows: Vec<AnyElement> = apps
      .iter()
      .map(|app| {
        let folder = self.settings.code_folders.get(&app.id).cloned();
        let missing = folder.as_ref().is_some_and(|folder| !folder.is_dir());
        let shown: SharedString = match &folder {
          Some(folder) => folder.display().to_string().into(),
          None => tr(l, "notSet"),
        };
        let (choose_id, clear_id) = (app.id.clone(), app.id.clone());
        let mut actions = h_flex().gap_2().child(
          Button::new(SharedString::from(format!("choose-folder-{}", app.id)))
            .icon(Icon::new(IconName::FolderOpen))
            .label(if folder.is_some() { tr(l, "change") } else { tr(l, "chooseFolder") })
            .outline()
            .small()
            .on_click(cx.listener(move |this, _, window, cx| this.choose_code_folder(choose_id.clone(), None, window, cx))),
        );
        if folder.is_some() {
          actions = actions.child(
            Button::new(SharedString::from(format!("clear-folder-{}", app.id)))
              .label(tr(l, "remove"))
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
              .children(missing.then(|| div().text_size(FONT_SIZE_SM).text_color(c.danger).child(tr(l, "thisFolderWasNot")))),
          )
          .child(actions)
          .into_any_element()
      })
      .collect();
    v_flex().gap_2().child(intro).child(v_flex().children(rows)).into_any_element()
  }

  fn settings_page_body(&mut self, wide: bool, cx: &mut Context<Self>) -> AnyElement {
    let l = self.settings.language;
    match self.settings_page {
      SettingsPage::General => v_flex()
        .child(setting_row(
          tr(l, "language"),
          Some(tr(l, "chooseTheLanguageOf")),
          div().w(px(FIELD_WIDTH)).max_w_full().child(Select::new(&self.language_select)),
          cx,
        ))
        .into_any_element(),
      SettingsPage::Account => {
        let account = match &self.settings.apple {
          Some(account) => trf(l, "signedInWithKey", &[&account.key_id]),
          None => tr(l, "notSignedIn"),
        };
        v_flex()
          .child(setting_row(
            tr(l, "account"),
            Some(account),
            Button::new("settings-sign-out").label(tr(l, "signOut")).outline().on_click(cx.listener(|this, _, window, cx| this.sign_out(window, cx))),
            cx,
          ))
          .into_any_element()
      }
      SettingsPage::AiProvider => {
        let [first, second] = Provider::ALL.map(|provider| self.provider_section(provider, cx).into_any_element());
        v_flex()
          .gap_8()
          .child(page_columns(wide, first, second))
          .child(self.usage_section(cx))
          .child(div().text_size(FONT_SIZE_SM).text_color(cx.theme().muted_foreground).child(tr(l, "moreProvidersCanBe")))
          .into_any_element()
      }
      SettingsPage::CodeFolders => self.code_folders_body(cx),
      SettingsPage::Rules => v_flex()
        .gap_4()
        .child(self.render_guidelines(cx))
        .child(setting_row(
          tr(l, "appStoreCompliancePlaybook"),
          Some(tr(l, "theAnalysisReadsThe")),
          Button::new("settings-playbook").label(tr(l, "openThePlaybookOn")).outline().on_click(|_, _, cx| cx.open_url(PLAYBOOK_REPOSITORY)),
          cx,
        ))
        .into_any_element(),
      SettingsPage::About => self.about_body(cx),
    }
  }

  pub(crate) fn render_settings(&mut self, wide: bool, cx: &mut Context<Self>) -> impl IntoElement {
    let l = self.settings.language;
    let muted = cx.theme().muted_foreground;
    let heading = v_flex().gap_1().child(div().text_xl().font_weight(FontWeight::SEMIBOLD).child(tr(l, self.settings_page.label()))).children(
      (self.settings_page == SettingsPage::AiProvider).then(|| div().text_size(FONT_SIZE_SM).text_color(muted).child(tr(l, "aiAnalysisUsesYour"))),
    );
    let split = wide && self.settings_page == SettingsPage::AiProvider;
    let body = self.settings_page_body(split, cx);
    let width = if split { rems(86.) } else { rems(62.) };
    v_flex()
      .id("settings")
      .flex_1()
      .min_h_0()
      .overflow_y_scrollbar()
      .child(centered_with_width(v_flex().py_6().gap_4().child(heading).child(body), width))
  }
}

/// 12345 as "12.3K", 2_500_000 as "2.50M".
fn tokens_text(tokens: u64) -> String {
  match tokens {
    0..=999 => tokens.to_string(),
    1_000..=999_999 => format!("{:.1}K", tokens as f64 / 1_000.),
    _ => format!("{:.2}M", tokens as f64 / 1_000_000.),
  }
}

/// Small amounts keep four decimals so a single request does not show as $0.00.
fn cost_text(cost: f64) -> String {
  if cost < 1. {
    format!("${cost:.4}")
  } else {
    format!("${cost:.2}")
  }
}

/// A Unix time in seconds as "2026-11-19" (UTC).
fn date_text(seconds: u64) -> String {
  // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
  let days = (seconds / 86_400) as i64 + 719_468;
  let era = days.div_euclid(146_097);
  let day_of_era = days.rem_euclid(146_097);
  let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
  let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
  let month_index = (5 * day_of_year + 2) / 153;
  let day = day_of_year - (153 * month_index + 2) / 5 + 1;
  let month = if month_index < 10 { month_index + 3 } else { month_index - 9 };
  let year = year_of_era + era * 400 + i64::from(month <= 2);
  format!("{year}-{month:02}-{day:02}")
}
