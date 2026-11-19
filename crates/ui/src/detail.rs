//! The open app's page: heading, the loaded body, and the App Information, App Review and App Privacy forms.

use crate::{
  i18n::{tr, trf},
  theme::colors,
  widgets::{area_field, area_field_with_action, centered, pair, section, text_field, text_field_with_action, url_field},
  workspace::{locales, DetailTab, Load, Workspace},
};
use appforge_appstore::App;
use appforge_compliance::ListingField;
use appforge_storage::PrivacyDeclaration;
use gpui_kit::{
  component::{
    alert::Alert,
    button::{Button, ButtonVariants as _},
    empty::{Empty, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle},
    h_flex,
    scroll::ScrollableElement as _,
    spinner::Spinner,
    switch::Switch,
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

fn app_privacy_url(app: &App) -> String {
  format!("https://appstoreconnect.apple.com/apps/{}/distribution/privacy", app.id)
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
    let c = colors(cx);
    let l = self.settings.language;
    let body = if self.tab == DetailTab::CustomerReviews {
      self.render_customer_reviews(cx)
    } else {
      match self.active_data().map(|data| &data.snapshot) {
        Some(Load::Ready(_)) => match self.tab {
          DetailTab::Distribution => self.render_distribution(cx),
          DetailTab::CustomerReviews => self.render_customer_reviews(cx),
          DetailTab::Information => self.render_information(&app, cx).into_any_element(),
          DetailTab::Review => self.render_review(cx).into_any_element(),
          DetailTab::Privacy => self.render_privacy(&app, cx).into_any_element(),
          DetailTab::Analysis => self.render_analysis(cx).into_any_element(),
        },
        Some(Load::Failed(message)) => v_flex()
          .gap_3()
          .child(Alert::error("snapshot-error", message.clone()))
          .child(
            h_flex().child(
              Button::new("retry-snapshot")
                .label(tr(l, "tryAgain"))
                .outline()
                .on_click(cx.listener(|this, _, window, cx| this.load_snapshot(window, cx))),
            ),
          )
          .into_any_element(),
        _ => {
          h_flex().gap_2().items_center().text_color(c.text_secondary).child(Spinner::new()).child(tr(l, "loadingFromAppStore")).into_any_element()
        }
      }
    };
    // The pages are listed in the sidebar, so the page says which one it is; it scrolls under that heading.
    let title = div().text_xl().font_weight(FontWeight::SEMIBOLD).child(tr(l, self.tab.label()));
    // After a jump from another page, a way back to it comes first.
    let heading = match self.back_tab {
      Some(origin) => h_flex()
        .gap_3()
        .items_center()
        .child(
          Button::new("back-to-page")
            .icon(Icon::new(IconName::ArrowLeft))
            .label(tr(l, origin.label()))
            .outline()
            .small()
            .tooltip(trf(l, "backTo", &[&tr(l, origin.label())]))
            .on_click(cx.listener(|this, _, window, cx| this.go_back(window, cx))),
        )
        .child(title)
        .into_any_element(),
      None => title.into_any_element(),
    };
    v_flex().flex_1().min_h_0().overflow_hidden().child(
      v_flex().id("detail-scroll").size_full().overflow_y_scrollbar().child(centered(v_flex().pt_6().pb_6().gap_6().child(heading).child(body))),
    )
  }

  /// The button that saves the page, shown in the sidebar. The Analysis page has none.
  pub(crate) fn save_control(&self, cx: &mut Context<Self>) -> Option<Button> {
    let l = self.settings.language;
    let has_version = self.ready_snapshot().is_some_and(|snapshot| snapshot.version.is_some());
    let tab = self.tab;
    let (id, label, enabled): (&'static str, SharedString, bool) = match tab {
      DetailTab::Information => ("save-information", tr(l, "save"), true),
      DetailTab::Review => ("save-review", tr(l, "save"), has_version),
      DetailTab::Privacy => ("save-privacy", tr(l, "save"), true),
      DetailTab::Analysis | DetailTab::Distribution | DetailTab::CustomerReviews => return None,
    };
    let tooltip = if tab == DetailTab::Privacy { tr(l, "yourAnswersStayOn") } else { tr(l, "saveToAppStore") };
    Some(
      self
        .save_button(id, label, enabled, cx, move |this, window, cx| match tab {
          DetailTab::Information => this.save_information(window, cx),
          DetailTab::Review => this.save_review(window, cx),
          DetailTab::Privacy => this.save_privacy(window, cx),
          DetailTab::Analysis | DetailTab::Distribution | DetailTab::CustomerReviews => {}
        })
        .tooltip(tooltip),
    )
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
  ) -> Button {
    Button::new(id)
      .label(label)
      .primary()
      .small()
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
    let mut page = v_flex().gap_6().children(picker);
    page = page.child(section(
      tr(l, "basics"),
      pair(
        text_field_with_action(tr(l, "appName"), &forms.name, NAME_LIMIT, self.field_generate_button(ListingField::Name, cx), cx),
        text_field_with_action(tr(l, "subtitle"), &forms.subtitle, SUBTITLE_LIMIT, self.field_generate_button(ListingField::Subtitle, cx), cx),
      ),
    ));
    if has_version {
      page = page
        .child(section(
          tr(l, "storeListing"),
          v_flex()
            .gap_4()
            .child(area_field_with_action(
              tr(l, "description"),
              &forms.description,
              DESCRIPTION_LIMIT,
              self.field_generate_button(ListingField::Description, cx),
              cx,
            ))
            .child(text_field_with_action(
              tr(l, "keywords"),
              &forms.keywords,
              KEYWORDS_LIMIT,
              self.field_generate_button(ListingField::Keywords, cx),
              cx,
            ))
            .child(area_field_with_action(
              tr(l, "promotionalText"),
              &forms.promotional,
              PROMOTIONAL_LIMIT,
              self.field_generate_button(ListingField::PromotionalText, cx),
              cx,
            )),
        ))
        .child(section(
          tr(l, "links"),
          pair(
            url_field("open-support-url", tr(l, "supportUrl"), &forms.support_url, l, cx),
            url_field("open-marketing-url", tr(l, "marketingUrl"), &forms.marketing_url, l, cx),
          ),
        ))
        .child(section(
          tr(l, "whatsNewInThis"),
          area_field_with_action(
            tr(l, "whatsNewInThis"),
            &forms.whats_new,
            WHATS_NEW_LIMIT,
            self.field_generate_button(ListingField::WhatsNew, cx),
            cx,
          ),
        ));
    } else {
      page = page.child(div().text_sm().text_color(theme.muted_foreground).child(tr(l, "thisAppHasNo")));
    }
    page
  }

  fn render_review(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
    let l = self.settings.language;
    let snapshot = self.ready_snapshot();
    let has_version = snapshot.is_some_and(|snapshot| snapshot.version.is_some());
    let missing =
      snapshot.is_some_and(|snapshot| snapshot.version.is_some() && snapshot.review_detail.as_ref().is_none_or(|detail| detail.id.is_none()));
    let editable = snapshot.and_then(|snapshot| snapshot.version.as_ref()).is_some_and(|version| version.is_editable());
    let forms = &self.forms;
    let mut page = v_flex().gap_6();
    if !has_version {
      page = page.child(Alert::warning("review-no-version", tr(l, "thisAppHasNoIos")));
    } else if !editable {
      page = page.child(Alert::info("review-locked", tr(l, "thisVersionIsNo")));
    }
    if missing {
      page = page.child(Alert::info("review-missing", tr(l, "nothingIsStoredFor")));
    }
    let mut sign_in = v_flex().gap_4().child(Switch::new("demo-required").checked(forms.demo_required).label(tr(l, "signInRequiredTo")).on_click(
      cx.listener(|this, checked: &bool, _, cx| {
        this.forms.demo_required = *checked;
        cx.notify();
      }),
    ));
    if forms.demo_required {
      sign_in = sign_in.child(pair(
        text_field(tr(l, "demoUserName"), &forms.demo_name, None, cx),
        text_field(tr(l, "demoPassword"), &forms.demo_password, None, cx),
      ));
    }
    page
      .child(section(
        tr(l, "contact"),
        v_flex()
          .gap_4()
          .child(pair(text_field(tr(l, "firstName"), &forms.first_name, None, cx), text_field(tr(l, "lastName"), &forms.last_name, None, cx)))
          .child(pair(text_field(tr(l, "phone"), &forms.phone, None, cx), text_field(tr(l, "email"), &forms.email, None, cx))),
      ))
      .child(section(tr(l, "signInHeading"), sign_in))
      .child(section(tr(l, "notesForTheReviewer"), area_field(tr(l, "notes"), &forms.notes, None, cx)))
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
      .gap_6()
      .child(Alert::info("privacy-api", tr(l, "applesApiDoesNot")))
      .child(
        h_flex().child(
          Button::new("open-app-privacy")
            .icon(Icon::new(IconName::ExternalLink))
            .label(tr(l, "openAppPrivacyIn"))
            .outline()
            .on_click(move |_, _, cx| cx.open_url(&url)),
        ),
      )
      .child(section(tr(l, "privacyPolicyUrl"), url_field("open-privacy-url", tr(l, "privacyPolicyUrl"), &forms.privacy_url, l, cx)))
      .child(section(
        tr(l, "yourAnswers"),
        v_flex()
          .gap_3()
          .child(toggle(
            "p-confirmed",
            tr(l, "iAnsweredAppPrivacy"),
            declaration.confirmed_in_app_store_connect,
            |d, v| d.confirmed_in_app_store_connect = v,
            cx,
          ))
          .child(toggle("p-collects", tr(l, "theAppCollectsData"), declaration.collects_data, |d, v| d.collects_data = v, cx))
          .child(toggle("p-tracking", tr(l, "theAppTracksUsers"), declaration.uses_tracking, |d, v| d.uses_tracking = v, cx))
          .child(toggle("p-att", tr(l, "theAppShowsThe"), declaration.shows_tracking_prompt, |d, v| d.shows_tracking_prompt = v, cx))
          .child(toggle("p-accounts", tr(l, "peopleCanCreateAn"), declaration.has_accounts, |d, v| d.has_accounts = v, cx))
          .child(toggle("p-deletion", tr(l, "accountsCanBeDeleted"), declaration.account_deletion_in_app, |d, v| d.account_deletion_in_app = v, cx))
          .child(toggle("p-login", tr(l, "aReviewerMustSign"), declaration.requires_login_to_review, |d, v| d.requires_login_to_review = v, cx))
          .child(pair(
            text_field(tr(l, "dataTypesCollected"), &forms.data_types, None, cx),
            text_field(tr(l, "thirdPartySdks"), &forms.sdks, None, cx),
          )),
      ))
  }
}
