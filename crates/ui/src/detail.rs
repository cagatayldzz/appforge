//! The open app's page: heading, the loaded body, and the App Information, App Review and App Privacy forms.

use crate::{
  i18n::{tr, trf},
  theme::colors,
  widgets::{
    area_field, area_field_with_action, centered_with_width, page_columns, page_columns_with_primary, page_section, pair, required, required_mark,
    section, text_field, text_field_with_action, url_field,
  },
  workspace::{DetailTab, Load, Workspace},
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
    skeleton::Skeleton,
    switch::Switch,
    v_flex, ActiveTheme, Disableable as _, Icon, IconName, Sizable as _,
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

  pub(crate) fn render_detail(&mut self, app: App, wide: bool, cx: &mut Context<Self>) -> impl IntoElement {
    let l = self.settings.language;
    let body = if self.tab == DetailTab::CustomerReviews {
      self.render_customer_reviews(cx)
    } else {
      match self.active_data().map(|data| &data.snapshot) {
        Some(Load::Ready(_)) => match self.tab {
          DetailTab::ProductPage => self.render_product(&app, wide, cx).into_any_element(),
          DetailTab::Distribution => self.render_distribution(wide, cx),
          DetailTab::CustomerReviews => self.render_customer_reviews(cx),
          DetailTab::Information => self.render_information(&app, cx).into_any_element(),
          DetailTab::Compliance => self.render_compliance(&app, cx).into_any_element(),
          DetailTab::Accessibility => self.render_store_only(&app, "accessibility", "appAccessibilityHelp", cx).into_any_element(),
          DetailTab::Submissions => self.render_review_submissions(&app, cx),
          DetailTab::Review => self.render_review(wide, cx).into_any_element(),
          DetailTab::Privacy => self.render_privacy(&app, wide, cx).into_any_element(),
          DetailTab::Analysis => self.render_analysis(wide, cx).into_any_element(),
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
        _ => self.detail_loading(wide, cx).into_any_element(),
      }
    };
    // The pages are listed in the sidebar, so the page says which one it is; the heading stays fixed while the body scrolls under it.
    let title = match (self.tab, self.ready_snapshot().and_then(|snapshot| snapshot.version.as_ref())) {
      (DetailTab::ProductPage, Some(version)) => trf(l, "iosAppVersion", &[&version.version_string]),
      _ => tr(l, self.tab.label()),
    };
    let title = div().text_xl().font_weight(FontWeight::SEMIBOLD).child(title);
    // The version page puts adding the version for review in the heading, as App Store Connect does.
    let review_action = if self.tab == DetailTab::ProductPage { self.distribution_parts(cx).ok().and_then(|parts| parts.action) } else { None };
    let actions = h_flex().gap_2().items_center().children(self.form_history_button(cx)).children(review_action);
    let heading = h_flex().py_4().gap_3().flex_wrap().items_center().justify_between().child(title).child(actions);
    let content = v_flex().pt_6().pb_6().gap_6().child(body);
    let max_width = match self.tab {
      DetailTab::ProductPage | DetailTab::CustomerReviews => rems(90.),
      _ if wide => rems(86.),
      _ => rems(60.),
    };
    let header = v_flex().flex_none().w_full().border_b_1().border_color(colors(cx).border).child(centered_with_width(heading, max_width));
    let content = centered_with_width(content, max_width);
    v_flex()
      .flex_1()
      .min_h_0()
      .overflow_hidden()
      .child(header)
      .child(v_flex().id("detail-scroll").flex_1().min_h_0().w_full().overflow_y_scrollbar().child(content))
  }

  fn detail_loading(&self, wide: bool, cx: &gpui_kit::App) -> Div {
    let c = colors(cx);
    let panel = || {
      v_flex()
        .w_full()
        .gap_3()
        .p_4()
        .rounded_lg()
        .border_1()
        .border_color(c.border)
        .child(Skeleton::new().h_4().w_32())
        .child(Skeleton::new().h_3().w_full().max_w_64())
        .child(Skeleton::new().h_3().w_24())
    };
    let primary = v_flex().gap_3().child(panel()).child(panel());
    let secondary = v_flex().gap_3().child(panel()).child(panel());
    let share = if self.tab == DetailTab::Information { 0.65 } else { 0.56 };
    v_flex()
      .gap_4()
      .child(div().text_sm().text_color(c.text_secondary).child(tr(self.settings.language, "loadingFromAppStore")))
      .child(page_columns_with_primary(wide, share, primary, secondary))
  }

  /// The button that saves the page, shown in the sidebar. The Analysis page has none.
  pub(crate) fn save_control(&self, cx: &mut Context<Self>) -> Option<Button> {
    let l = self.settings.language;
    let id = match self.effective_tab() {
      DetailTab::Information => "save-information",
      DetailTab::ProductPage => "save-product-page",
      DetailTab::Review => "save-review",
      DetailTab::Privacy => "save-privacy",
      _ => return None,
    };
    let dirty = self.has_pending_save(cx);
    Some(
      Button::new(id)
        .label(tr(l, "save"))
        .primary()
        .small()
        .loading(self.saving)
        .disabled(!dirty || self.saving || self.store_write_busy || self.settings_locked)
        .tooltip(tr(l, if dirty { "reviewSaveChanges" } else { "nothingChanged" }))
        .on_click(cx.listener(|this, _, window, cx| this.confirm_save(window, cx))),
    )
  }

  fn compact_locale_picker(&self, cx: &mut Context<Self>) -> Option<Div> {
    let label = tr(self.settings.language, "storeListingLanguage");
    let c = colors(cx);
    Some(h_flex().gap_2().flex_wrap().items_center().child(div().text_sm().text_color(c.text_secondary).child(label)).child(self.locale_picker(cx)?))
  }

  /// App Information as App Store Connect lays it out, one page that scrolls: the intro, the compliance notice, Localizable Information,
  /// then what `render_general` lists.
  fn render_information(&mut self, app: &App, cx: &mut Context<Self>) -> impl IntoElement {
    let l = self.settings.language;
    let c = colors(cx);
    let picker = self.compact_locale_picker(cx);
    let forms = &self.forms;
    let general = self.render_general(app, cx);
    let notice = v_flex().gap_2().child(Alert::info("regulatory-notice", tr(l, "regulatoryNotice"))).child(
      div()
        .id("go-to-compliance")
        .cursor_pointer()
        .text_size(crate::theme::FONT_SIZE_SM)
        .text_color(c.accent)
        .child(tr(l, "goToAppCompliance"))
        .on_click(cx.listener(|this, _, window, cx| this.jump_to(DetailTab::Compliance, window, cx))),
    );
    let basics = page_section(
      tr(l, "basics"),
      v_flex().gap_4().children(picker).child(pair(
        text_field_with_action(required(tr(l, "appName")), &forms.name, NAME_LIMIT, self.field_generate_button(ListingField::Name, cx), cx),
        text_field_with_action(tr(l, "subtitle"), &forms.subtitle, SUBTITLE_LIMIT, self.field_generate_button(ListingField::Subtitle, cx), cx),
      )),
    );
    v_flex().gap_6().child(div().text_color(c.text_secondary).child(tr(l, "appInformationHelp"))).child(notice).child(basics).child(general)
  }

  /// The version page as App Store Connect lays it out, one page that scrolls: the notices, Product Page Information (with its
  /// listing language and the media), Build, App Review Information and App Store Version Release.
  fn render_product(&mut self, app: &App, wide: bool, cx: &mut Context<Self>) -> impl IntoElement {
    let l = self.settings.language;
    let c = colors(cx);
    let heading = |title: SharedString, right: Option<AnyElement>| {
      h_flex().w_full().gap_3().items_center().justify_between().child(div().text_xl().font_weight(FontWeight::SEMIBOLD).child(title)).children(right)
    };
    let rule = || div().w_full().h_px().bg(c.border);
    // The rejection notice comes first, as in App Store Connect.
    let rejection = self.rejection_notice(app, cx);
    let picker = self.locale_picker(cx).map(IntoElement::into_any_element);
    let listing = self.render_listing(cx);
    let media = self.render_product_page(app, cx);
    let review = self.render_review(wide, cx);
    let parts = self.distribution_parts(cx);
    let mut page = v_flex().gap_6().children(rejection);
    page = page.child(
      v_flex()
        .gap_3()
        .child(heading(tr(l, "productPageInformation"), picker))
        .child(div().text_sm().text_color(c.text_secondary).child(tr(l, "productPageHelp"))),
    );
    page = page.child(listing).child(section(tr(l, "mediaTab"), media));
    match parts {
      Err(message) => page = page.child(rule()).child(message),
      Ok(parts) => {
        if let Some(rest) = parts.rest {
          page = page
            .child(rule())
            .child(v_flex().gap_3().child(heading(tr(l, "builds"), None)).child(rest.builds))
            .child(rule())
            .child(v_flex().gap_3().child(heading(tr(l, "appReviewInformation"), None)).child(review))
            .child(rule())
            .child(v_flex().gap_3().child(heading(tr(l, "appStoreVersionRelease"), None)).child(rest.release));
        } else {
          page = page.child(rule()).child(v_flex().gap_3().child(heading(tr(l, "appReviewInformation"), None)).child(review));
        }
      }
    }
    page
  }

  fn render_listing(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
    let l = self.settings.language;
    let version = self.ready_snapshot().and_then(|snapshot| snapshot.version.as_ref());
    let theme = cx.theme().clone();
    let forms = &self.forms;
    // The fields in the order of App Store Connect, without headings of our own.
    let mut page = v_flex().gap_4();
    if let Some(version) = version {
      if version.supports_release_notes {
        page = page.child(area_field_with_action(
          required(tr(l, "whatsNewInThis")),
          &forms.whats_new,
          WHATS_NEW_LIMIT,
          self.field_generate_button(ListingField::WhatsNew, cx),
          cx,
        ));
      }
      page = page
        .child(area_field_with_action(
          tr(l, "promotionalText"),
          &forms.promotional,
          PROMOTIONAL_LIMIT,
          self.field_generate_button(ListingField::PromotionalText, cx),
          cx,
        ))
        .child(area_field_with_action(
          required(tr(l, "description")),
          &forms.description,
          DESCRIPTION_LIMIT,
          self.field_generate_button(ListingField::Description, cx),
          cx,
        ))
        .child(text_field_with_action(
          required(tr(l, "keywords")),
          &forms.keywords,
          KEYWORDS_LIMIT,
          self.field_generate_button(ListingField::Keywords, cx),
          cx,
        ))
        .child(pair(
          url_field("open-support-url", required(tr(l, "supportUrl")), &forms.support_url, l, cx),
          url_field("open-marketing-url", tr(l, "marketingUrl"), &forms.marketing_url, l, cx),
        ));
    } else {
      page = page.child(div().text_sm().text_color(theme.muted_foreground).child(tr(l, "thisAppHasNo")));
    }
    v_flex().gap_6().child(page)
  }

  fn render_review(&mut self, wide: bool, cx: &mut Context<Self>) -> impl IntoElement {
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
    let sign_in_required = forms.privacy.requires_login_to_review;
    let mut sign_in = v_flex().gap_4().child(
      h_flex()
        .gap_1()
        .items_center()
        .child(Switch::new("demo-required").checked(forms.demo_required).label(tr(l, "signInRequiredTo")).on_click(cx.listener(
          |this, checked: &bool, _, cx| {
            this.forms.demo_required = *checked;
            cx.notify();
          },
        )))
        .children(sign_in_required.then(|| required_mark(cx))),
    );
    if self.review_credentials_required() {
      sign_in = sign_in.child(pair(
        text_field(required(tr(l, "demoUserName")), &forms.demo_name, None, cx),
        text_field(required(tr(l, "demoPassword")), &forms.demo_password, None, cx),
      ));
    }
    let account = v_flex()
      .gap_6()
      .child(section(
        tr(l, "contact"),
        v_flex()
          .gap_4()
          .child(pair(
            text_field(required(tr(l, "firstName")), &forms.first_name, None, cx),
            text_field(required(tr(l, "lastName")), &forms.last_name, None, cx),
          ))
          .child(pair(text_field(required(tr(l, "phone")), &forms.phone, None, cx), text_field(required(tr(l, "email")), &forms.email, None, cx))),
      ))
      .child(section(tr(l, "signInHeading"), sign_in));
    let notes = area_field(tr(l, "notes"), &forms.notes, None, cx);
    page.child(page_columns_with_primary(wide, 0.40, account, notes))
  }

  fn render_privacy(&mut self, app: &App, wide: bool, cx: &mut Context<Self>) -> impl IntoElement {
    let l = self.settings.language;
    let declaration = self.privacy_draft(cx);
    let url = app_privacy_url(app);
    let picker = self.compact_locale_picker(cx);
    let forms = &self.forms;
    let toggle = |id: &'static str, label: SharedString, checked: bool, change: fn(&mut PrivacyDeclaration, bool), cx: &mut Context<Self>| {
      Switch::new(id).checked(checked).label(label).on_click(cx.listener(move |this, value: &bool, _, cx| {
        change(&mut this.forms.privacy, *value);
        cx.notify();
      }))
    };
    let answers = section(
      tr(l, "yourAnswers"),
      v_flex()
        .gap_3()
        .child(
          h_flex()
            .gap_1()
            .items_center()
            .child(toggle(
              "p-confirmed",
              tr(l, "iAnsweredAppPrivacy"),
              declaration.confirmed_in_app_store_connect,
              |d, v| d.confirmed_in_app_store_connect = v,
              cx,
            ))
            .child(required_mark(cx)),
        )
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
    );
    let policy = v_flex()
      .gap_6()
      .child(section(
        tr(l, "privacyLinks"),
        v_flex()
          .gap_3()
          .children(picker)
          .child(url_field("open-privacy-url", required(tr(l, "privacyPolicyUrl")), &forms.privacy_url, l, cx))
          .child(url_field("open-privacy-choices", tr(l, "privacyChoicesUrl"), &forms.privacy_choices, l, cx)),
      ))
      .child(section(tr(l, "productPagePreview"), self.render_privacy_preview(&declaration, cx)));
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
      .child(page_columns(wide, answers, policy))
  }
}
