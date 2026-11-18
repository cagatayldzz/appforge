use crate::{
  i18n::{tr, trf},
  widgets::{area_field, page_title, text_field, CONTENT_MAX_WIDTH},
  workspace::{locales, DetailTab, Load, Workspace},
};
use appforge_appstore::App;
use appforge_storage::PrivacyDeclaration;
use gpui_kit::{
  component::{
    alert::Alert,
    button::{Button, ButtonVariants as _},
    empty::{Empty, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle},
    h_flex,
    spinner::Spinner,
    switch::Switch,
    tab::{Tab, TabBar},
    v_flex, ActiveTheme, Disableable as _, Icon, IconName, Selectable as _, Sizable as _,
  },
  *,
};

const NAME_LIMIT: usize = 30;
const SUBTITLE_LIMIT: usize = 30;
const KEYWORDS_LIMIT: usize = 100;
const PROMOTIONAL_LIMIT: usize = 170;
const DESCRIPTION_LIMIT: usize = 4000;
const WHATS_NEW_LIMIT: usize = 4000;

/// "PREPARE_FOR_SUBMISSION" as "Prepare for submission".
fn state_label(state: &str) -> String {
  let lower = state.replace('_', " ").to_lowercase();
  let mut characters = lower.chars();
  characters.next().map(|first| first.to_uppercase().collect::<String>() + characters.as_str()).unwrap_or_default()
}

fn app_privacy_url(app: &App) -> String {
  format!("https://appstoreconnect.apple.com/apps/{}/appPrivacy", app.id)
}

impl Workspace {
  pub(crate) fn render_message(&self, title: impl Into<SharedString>, text: impl Into<SharedString>, cx: &mut Context<Self>) -> impl IntoElement {
    let _ = cx;
    div().flex_1().flex().p_8().child(
      Empty::new().header(
        EmptyHeader::new()
          .media(EmptyMedia::new().child(Icon::new(IconName::LayoutDashboard)))
          .title(EmptyTitle::new().child(title.into()))
          .description(EmptyDescription::new().child(text.into())),
      ),
    )
  }

  pub(crate) fn render_detail(&mut self, app: App, cx: &mut Context<Self>) -> impl IntoElement {
    let theme = cx.theme().clone();
    let l = self.settings.language;
    let subtitle = match self.ready_snapshot().and_then(|snapshot| snapshot.version.as_ref()) {
      Some(version) => trf(l, "{} · version {} · {}", &[&app.bundle_id, &version.version_string, &state_label(&version.state)]).to_string(),
      None => app.bundle_id.clone(),
    };
    let selected_tab = DetailTab::ALL.iter().position(|tab| *tab == self.tab).unwrap_or(0);
    let tabs = TabBar::new("detail-tabs")
      .underline()
      .selected_index(selected_tab)
      .on_click(cx.listener(|this, index: &usize, _, cx| {
        this.tab = DetailTab::ALL[*index];
        cx.notify();
      }))
      .children(DetailTab::ALL.iter().map(|tab| Tab::new().label(tr(l, tab.label()))));
    let body = match &self.snapshot {
      Load::Ready(_) => match self.tab {
        DetailTab::Information => self.render_information(&app, cx).into_any_element(),
        DetailTab::Review => self.render_review(cx).into_any_element(),
        DetailTab::Privacy => self.render_privacy(&app, cx).into_any_element(),
        DetailTab::Analysis => self.render_analysis(cx).into_any_element(),
      },
      Load::Failed(message) => v_flex()
        .gap_3()
        .child(Alert::error("snapshot-error", message.clone()))
        .child(
          Button::new("retry-snapshot")
            .label(tr(l, "Try again"))
            .outline()
            .on_click(cx.listener(|this, _, window, cx| this.load_snapshot(window, cx))),
        )
        .into_any_element(),
      _ => h_flex()
        .gap_2()
        .items_center()
        .text_sm()
        .text_color(theme.muted_foreground)
        .child(Spinner::new())
        .child(tr(l, "Loading from App Store Connect…"))
        .into_any_element(),
    };
    v_flex()
      .w_full()
      .max_w(px(CONTENT_MAX_WIDTH))
      .px_8()
      .pt_4()
      .pb_8()
      .gap_4()
      .child(
        h_flex().justify_between().items_start().child(page_title(app.name.clone(), subtitle, cx)).child(
          Button::new("reload-snapshot")
            .icon(Icon::new(IconName::Redo))
            .label(tr(l, "Reload"))
            .ghost()
            .on_click(cx.listener(|this, _, window, cx| this.load_snapshot(window, cx))),
        ),
      )
      .child(tabs)
      .child(body)
  }

  fn locale_picker(&mut self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
    let available = locales(self.ready_snapshot()?);
    if available.len() < 2 {
      return None;
    }
    Some(h_flex().gap_1().flex_wrap().children(available.into_iter().map(|locale| {
      let selected = locale == self.locale;
      let target = locale.clone();
      Button::new(SharedString::from(format!("locale-{locale}")))
        .label(locale)
        .small()
        .outline()
        .selected(selected)
        .on_click(cx.listener(move |this, _, window, cx| this.select_locale(target.clone(), window, cx)))
    })))
  }

  fn save_button(
    &self,
    id: &'static str,
    label: SharedString,
    enabled: bool,
    cx: &mut Context<Self>,
    save: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
  ) -> impl IntoElement {
    Button::new(id)
      .label(label)
      .primary()
      .loading(self.saving)
      .disabled(!enabled || self.saving || self.settings_locked)
      .on_click(cx.listener(move |this, _, window, cx| save(this, window, cx)))
  }

  fn render_information(&mut self, app: &App, cx: &mut Context<Self>) -> impl IntoElement {
    let _ = app;
    let l = self.settings.language;
    let has_version = self.ready_snapshot().is_some_and(|snapshot| snapshot.version.is_some());
    let picker = self.locale_picker(cx);
    let theme = cx.theme().clone();
    let forms = &self.forms;
    let mut page = v_flex().gap_4().children(picker);
    page = page.child(text_field(tr(l, "App name"), &forms.name, Some(NAME_LIMIT), cx)).child(text_field(
      tr(l, "Subtitle"),
      &forms.subtitle,
      Some(SUBTITLE_LIMIT),
      cx,
    ));
    if has_version {
      page = page
        .child(div().pt_2().text_lg().font_weight(FontWeight::SEMIBOLD).child(tr(l, "Store listing")))
        .child(area_field(tr(l, "Description"), &forms.description, Some(DESCRIPTION_LIMIT), cx))
        .child(text_field(tr(l, "Keywords"), &forms.keywords, Some(KEYWORDS_LIMIT), cx))
        .child(area_field(tr(l, "Promotional text"), &forms.promotional, Some(PROMOTIONAL_LIMIT), cx))
        .child(text_field(tr(l, "Support URL"), &forms.support_url, None, cx))
        .child(text_field(tr(l, "Marketing URL"), &forms.marketing_url, None, cx))
        .child(area_field(tr(l, "What's new in this version"), &forms.whats_new, Some(WHATS_NEW_LIMIT), cx));
    } else {
      page = page.child(
        div()
          .text_sm()
          .text_color(theme.muted_foreground)
          .child(tr(l, "This app has no iOS version in preparation, so the store listing cannot be edited. Create a version in App Store Connect.")),
      );
    }
    page
      .child(self.save_button("save-information", tr(l, "Save to App Store Connect"), true, cx, |this, window, cx| this.save_information(window, cx)))
  }

  fn render_review(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
    let l = self.settings.language;
    let snapshot = self.ready_snapshot();
    let has_version = snapshot.is_some_and(|snapshot| snapshot.version.is_some());
    let missing =
      snapshot.is_some_and(|snapshot| snapshot.version.is_some() && snapshot.review_detail.as_ref().is_none_or(|detail| detail.id.is_none()));
    let editable = snapshot.and_then(|snapshot| snapshot.version.as_ref()).is_some_and(|version| version.is_editable());
    let forms = &self.forms;
    let mut page = v_flex().gap_4();
    if !has_version {
      page = page
        .child(Alert::warning("review-no-version", tr(l, "This app has no iOS version in preparation, so App Review information cannot be saved.")));
    } else if !editable {
      page = page.child(Alert::info("review-locked", tr(l, "This version is no longer editable at Apple, so saving may be refused.")));
    }
    if missing {
      page = page.child(Alert::info("review-missing", tr(l, "Nothing is stored for App Review yet. Saving creates it.")));
    }
    page = page
      .child(div().text_lg().font_weight(FontWeight::SEMIBOLD).child(tr(l, "Contact")))
      .child(
        h_flex()
          .gap_4()
          .child(div().flex_1().child(text_field(tr(l, "First name"), &forms.first_name, None, cx)))
          .child(div().flex_1().child(text_field(tr(l, "Last name"), &forms.last_name, None, cx))),
      )
      .child(
        h_flex().gap_4().child(div().flex_1().child(text_field(tr(l, "Phone"), &forms.phone, None, cx))).child(div().flex_1().child(text_field(
          tr(l, "Email"),
          &forms.email,
          None,
          cx,
        ))),
      )
      .child(div().pt_2().text_lg().font_weight(FontWeight::SEMIBOLD).child(tr(l, "Sign-in")))
      .child(Switch::new("demo-required").checked(forms.demo_required).label(tr(l, "Sign-in required to review the app")).on_click(cx.listener(
        |this, checked: &bool, _, cx| {
          this.forms.demo_required = *checked;
          cx.notify();
        },
      )));
    if forms.demo_required {
      page = page.child(
        h_flex()
          .gap_4()
          .child(div().flex_1().child(text_field(tr(l, "Demo user name"), &forms.demo_name, None, cx)))
          .child(div().flex_1().child(text_field(tr(l, "Demo password"), &forms.demo_password, None, cx))),
      );
    }
    page
      .child(div().pt_2().text_lg().font_weight(FontWeight::SEMIBOLD).child(tr(l, "Notes for the reviewer")))
      .child(area_field(tr(l, "Notes"), &forms.notes, None, cx))
      .child(self.save_button("save-review", tr(l, "Save to App Store Connect"), has_version, cx, |this, window, cx| this.save_review(window, cx)))
  }

  fn render_privacy(&mut self, app: &App, cx: &mut Context<Self>) -> impl IntoElement {
    let l = self.settings.language;
    let declaration = self.privacy_declaration();
    let url = app_privacy_url(app);
    let forms = &self.forms;
    let toggle = |id: &'static str, label: SharedString, checked: bool, change: fn(&mut PrivacyDeclaration, bool), cx: &mut Context<Self>| {
      Switch::new(id).checked(checked).label(label).on_click(cx.listener(move |this, value: &bool, window, cx| {
        let value = *value;
        this.update_declaration(window, cx, |declaration| change(declaration, value));
      }))
    };
    v_flex()
      .gap_4()
      .child(Alert::info(
        "privacy-api",
        tr(l, "Apple's API does not expose the App Privacy (nutrition label) answers, so they are edited in App Store Connect. The privacy policy URL below is saved through the API. The answers on this page stay on your Mac and feed the Analysis."),
      ))
      .child(
        Button::new("open-app-privacy")
          .icon(Icon::new(IconName::ExternalLink))
          .label(tr(l, "Open App Privacy in App Store Connect"))
          .outline()
          .on_click(move |_, _, cx| cx.open_url(&url)),
      )
      .child(text_field(tr(l, "Privacy policy URL"), &forms.privacy_url, None, cx))
      .child(div().pt_2().text_lg().font_weight(FontWeight::SEMIBOLD).child(tr(l, "Your answers")))
      .child(toggle("p-confirmed", tr(l, "I answered App Privacy in App Store Connect"), declaration.confirmed_in_app_store_connect, |d, v| d.confirmed_in_app_store_connect = v, cx))
      .child(toggle("p-collects", tr(l, "The app collects data from users"), declaration.collects_data, |d, v| d.collects_data = v, cx))
      .child(toggle("p-tracking", tr(l, "The app tracks users across other companies' apps or sites"), declaration.uses_tracking, |d, v| d.uses_tracking = v, cx))
      .child(toggle("p-att", tr(l, "The app shows the App Tracking Transparency prompt"), declaration.shows_tracking_prompt, |d, v| d.shows_tracking_prompt = v, cx))
      .child(toggle("p-accounts", tr(l, "People can create an account in the app"), declaration.has_accounts, |d, v| d.has_accounts = v, cx))
      .child(toggle("p-deletion", tr(l, "Accounts can be deleted inside the app"), declaration.account_deletion_in_app, |d, v| d.account_deletion_in_app = v, cx))
      .child(toggle("p-login", tr(l, "A reviewer must sign in to reach the main features"), declaration.requires_login_to_review, |d, v| d.requires_login_to_review = v, cx))
      .child(text_field(tr(l, "Data types collected"), &forms.data_types, None, cx))
      .child(text_field(tr(l, "Third-party SDKs"), &forms.sdks, None, cx))
      .child(self.save_button("save-privacy", tr(l, "Save"), true, cx, |this, window, cx| this.save_privacy(window, cx)))
  }
}
