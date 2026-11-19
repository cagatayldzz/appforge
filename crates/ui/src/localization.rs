//! Localized metadata selection and creation of missing App Store languages.

use crate::{
  forms::{read_input, set_input},
  i18n::{tr, trf},
  product_page::{store_info, store_version},
  widgets::text_field,
  workspace::{locales, DetailTab, Workspace},
};
use gpui_kit::{
  component::{
    button::{Button, ButtonVariants as _},
    h_flex,
    input::InputState,
    menu::{DropdownMenu as _, PopupMenuItem},
    v_flex, Icon, IconName, Sizable as _, WindowExt as _,
  },
  *,
};

const LOCALES: &[(&str, &str)] = &[
  ("ar-SA", "Arabic"),
  ("bn", "Bangla"),
  ("ca", "Catalan"),
  ("zh-Hans", "Chinese (Simplified)"),
  ("zh-Hant", "Chinese (Traditional)"),
  ("hr", "Croatian"),
  ("cs", "Czech"),
  ("da", "Danish"),
  ("nl-NL", "Dutch"),
  ("en-AU", "English (Australia)"),
  ("en-CA", "English (Canada)"),
  ("en-GB", "English (U.K.)"),
  ("en-US", "English (U.S.)"),
  ("fi", "Finnish"),
  ("fr-FR", "French"),
  ("fr-CA", "French (Canada)"),
  ("de-DE", "German"),
  ("el", "Greek"),
  ("gu", "Gujarati"),
  ("he", "Hebrew"),
  ("hi", "Hindi"),
  ("hu", "Hungarian"),
  ("id", "Indonesian"),
  ("it", "Italian"),
  ("ja", "Japanese"),
  ("kn", "Kannada"),
  ("ko", "Korean"),
  ("ms", "Malay"),
  ("ml", "Malayalam"),
  ("mr", "Marathi"),
  ("no", "Norwegian"),
  ("or", "Odia"),
  ("pl", "Polish"),
  ("pt-BR", "Portuguese (Brazil)"),
  ("pt-PT", "Portuguese (Portugal)"),
  ("pa", "Punjabi"),
  ("ro", "Romanian"),
  ("ru", "Russian"),
  ("sk", "Slovak"),
  ("sl", "Slovenian"),
  ("es-MX", "Spanish (Mexico)"),
  ("es-ES", "Spanish (Spain)"),
  ("sv", "Swedish"),
  ("ta", "Tamil"),
  ("te", "Telugu"),
  ("th", "Thai"),
  ("tr", "Turkish"),
  ("uk", "Ukrainian"),
  ("ur", "Urdu"),
  ("vi", "Vietnamese"),
];

pub(crate) fn locale_name(code: &str) -> &str {
  LOCALES.iter().find(|(known, _)| *known == code).map_or(code, |(_, name)| name)
}

impl Workspace {
  pub(crate) fn locale_picker(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
    let snapshot = self.ready_snapshot()?;
    let l = self.settings.language;
    let available = locales(snapshot);
    let primary = snapshot.app.primary_locale.clone();
    let selected = self.locale.clone();
    let add_url = if self.tab == DetailTab::ProductPage { store_version(&snapshot.app) } else { store_info(&snapshot.app) };
    let can_add = snapshot.version.as_ref().is_some_and(|version| version.is_editable()) && !snapshot.general.id.is_empty();
    let workspace = cx.weak_entity();
    let title = locale_name(if selected.is_empty() { &primary } else { &selected });
    Some(
      Button::new("listing-locales")
        .label(SharedString::from(title.to_owned()))
        .icon(Icon::new(IconName::ChevronDown))
        .outline()
        .small()
        .dropdown_menu(move |menu, _, _| {
          let mut menu = menu.scrollable(true).label(tr(l, "localized"));
          for code in &available {
            let name = locale_name(code);
            let label = if *code == primary { trf(l, "primaryLocaleLabel", &[name]) } else { name.to_owned().into() };
            let code = code.clone();
            let workspace = workspace.clone();
            menu = menu.item(PopupMenuItem::new(label).checked(code == selected).on_click(move |_, window, cx| {
              workspace.update(cx, |this, cx| this.select_locale(code.clone(), window, cx)).ok();
            }));
          }
          menu = menu.separator().label(tr(l, if can_add { "notLocalized" } else { "addLanguageInConnect" }));
          for &(code, name) in LOCALES {
            if !available.iter().any(|locale| locale == code) {
              if can_add {
                let workspace = workspace.clone();
                menu = menu.item(PopupMenuItem::new(name).on_click(move |_, window, cx| {
                  workspace.update(cx, |this, cx| this.open_add_localization(code, window, cx)).ok();
                }));
              } else {
                menu = menu.link(name, add_url.clone());
              }
            }
          }
          menu
        }),
    )
  }

  fn open_add_localization(&self, locale: &'static str, window: &mut Window, cx: &mut Context<Self>) {
    let Some(snapshot) = self.ready_snapshot() else { return };
    let Some(version) = snapshot.version.as_ref().filter(|version| version.is_editable()) else { return };
    let app = snapshot.app.id.clone();
    let info = snapshot.general.id.clone();
    let version = version.id.clone();
    let primary = snapshot.info_localizations.iter().find(|item| item.locale == snapshot.app.primary_locale);
    let name = cx.new(|cx| InputState::new(window, cx));
    let privacy_url = cx.new(|cx| InputState::new(window, cx));
    set_input(&name, primary.map_or(&snapshot.app.name, |item| &item.name), window, cx);
    if let Some(primary) = primary {
      set_input(&privacy_url, &primary.privacy_policy_url, window, cx);
    }
    let workspace = cx.weak_entity();
    let l = self.settings.language;
    window.open_dialog(cx, move |dialog, _, cx| {
      let (workspace, name_input, url_input) = (workspace.clone(), name.clone(), privacy_url.clone());
      let (app, info, version) = (app.clone(), info.clone(), version.clone());
      dialog
        .title(trf(l, "addLocalization", &[locale_name(locale)]))
        .child(v_flex().gap_3().child(text_field(tr(l, "appName"), &name, Some(30), cx)).child(text_field(
          tr(l, "privacyPolicyUrl"),
          &privacy_url,
          None,
          cx,
        )))
        .footer(
          h_flex()
            .gap_2()
            .justify_end()
            .child(Button::new("cancel-localization").label(tr(l, "cancel")).outline().on_click(|_, window, cx| window.close_dialog(cx)))
            .child(Button::new("create-localization").label(tr(l, "save")).primary().on_click(move |_, window, cx| {
              let name = read_input(&name_input, cx);
              let url = read_input(&url_input, cx);
              workspace
                .update(cx, |this, cx| {
                  if this.selected.as_ref() != Some(&app) || this.saving || this.store_write_busy {
                    return;
                  }
                  if !(2..=30).contains(&name.chars().count()) || !(url.starts_with("https://") || url.starts_with("http://")) {
                    Self::error(window, cx, tr(l, "localizationRequiredFields"));
                    return;
                  }
                  let (info, version) = (info.clone(), version.clone());
                  window.close_dialog(cx);
                  this.run_save(window, cx, move |client| client.add_localization(&info, &version, locale, &name, &url));
                })
                .ok();
            })),
        )
    });
  }
}
