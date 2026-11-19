//! A single checklist for submission, with links to the responsible screen.

use crate::{
  i18n::{tr, translated_text, trf},
  product_page::{store_info, store_link},
  theme::colors,
  widgets::{card, page_columns, section},
  workspace::{DetailTab, Load, Workspace},
};
use appforge_compliance::{Category, Severity};
use appforge_storage::PrivacyDeclaration;
use gpui_kit::{
  component::{button::Button, h_flex, tag::Tag, v_flex, Sizable as _, WindowExt as _},
  *,
};

impl Workspace {
  pub(crate) fn render_readiness(&self, wide: bool, cx: &mut Context<Self>) -> impl IntoElement {
    let l = self.settings.language;
    let c = colors(cx);
    let Some(snapshot) = self.ready_snapshot() else { return v_flex() };
    let distribution = self.active_data().map(|data| &data.distribution);
    let release_status = match (snapshot.version.as_ref(), distribution) {
      (None, _) => tr(l, "thisAppHasNoIos"),
      (Some(_), Some(Load::Ready(data))) if data.selected_build.is_none() => tr(l, "nextSelectABuild"),
      (Some(_), Some(Load::Ready(_))) => tr(l, "buildAttached"),
      (_, Some(Load::Failed(message))) => message.clone(),
      _ => tr(l, "loadingFromAppStore"),
    };
    let requirements = section(
      tr(l, "submissionRequirements"),
      v_flex()
        .gap_3()
        .child(card(
          tr(l, "distribution"),
          Some(release_status),
          Some(self.readiness_link(DetailTab::Distribution, cx).into_any_element()),
          div(),
          &c,
        ))
        .child(card(tr(l, "productPageInformation"), None, Some(self.readiness_link(DetailTab::ProductPage, cx).into_any_element()), div(), &c))
        .child(card(
          tr(l, "ageRatings"),
          Some(tr(
            l,
            if snapshot.general.ratings.is_empty() && snapshot.general.territory_ratings.is_empty() {
              "verifyAgeRating"
            } else {
              "ageRatingAvailable"
            },
          )),
          Some(store_link("readiness-age-rating", tr(l, "manageAgeRatings"), store_info(&snapshot.app)).into_any_element()),
          div(),
          &c,
        )),
    );
    let mut checks = v_flex().gap_3();
    for (category, tab) in
      [(Category::AppInformation, DetailTab::Information), (Category::AppReview, DetailTab::Review), (Category::AppPrivacy, DetailTab::Privacy)]
    {
      let mut findings = v_flex().gap_2();
      let mut count = 0;
      if let Some(report) = &self.report {
        for finding in report.findings.iter().filter(|item| item.category == category && item.severity != Severity::Info) {
          count += 1;
          let label = tr(l, finding.severity.label());
          let tag = match finding.severity {
            Severity::Critical => Tag::danger(),
            _ => Tag::warning(),
          };
          findings = findings.child(
            h_flex().gap_2().items_start().child(tag.small().child(label)).child(
              v_flex()
                .min_w_0()
                .gap_1()
                .child(div().font_weight(FontWeight::MEDIUM).child(translated_text(l, &finding.title)))
                .child(div().text_sm().text_color(c.text_secondary).child(translated_text(l, &finding.fix))),
            ),
          );
        }
      }
      if count == 0 {
        findings = findings.child(div().text_sm().text_color(c.text_secondary).child(tr(l, "noChecklistFindings")));
      }
      checks = checks.child(card(tr(l, tab.label()), None, Some(self.readiness_link(tab, cx).into_any_element()), findings, &c));
    }
    let conditions = section(
      tr(l, "conditionalRequirements"),
      v_flex().gap_2().child(div().text_sm().text_color(c.text_secondary).child(tr(l, "verifyMarketRequirements"))).child(
        h_flex()
          .gap_2()
          .flex_wrap()
          .child(store_link(
            "app-compliance",
            tr(l, "openAppCompliance"),
            format!("https://appstoreconnect.apple.com/apps/{}/distribution/app-compliance", snapshot.app.id),
          ))
          .child(self.readiness_link(DetailTab::Analysis, cx)),
      ),
    );
    v_flex().gap_6().child(page_columns(wide, v_flex().gap_6().child(requirements).child(conditions), checks))
  }

  fn readiness_link(&self, tab: DetailTab, cx: &mut Context<Self>) -> Button {
    let l = self.settings.language;
    Button::new(SharedString::from(format!("checklist-{}", tab.label())))
      .small()
      .outline()
      .label(trf(l, "openPage", &[&tr(l, tab.label())]))
      .on_click(cx.listener(move |this, _, window, cx| this.jump_to(tab, window, cx)))
  }

  pub(crate) fn render_privacy_preview(&self, declaration: &PrivacyDeclaration, cx: &mut Context<Self>) -> impl IntoElement {
    let l = self.settings.language;
    let c = colors(cx);
    let data: SharedString = declaration.data_types.clone().into();
    let details = tr(l, "localPrivacyPreviewHelp");
    let title = if !declaration.confirmed_in_app_store_connect {
      tr(l, "privacyAnswersUnconfirmed")
    } else if declaration.collects_data {
      tr(l, "dataCollectedPreview")
    } else {
      tr(l, "noDataCollectedPreview")
    };
    let body = v_flex()
      .gap_2()
      .child(div().text_sm().text_color(c.text_secondary).child(details.clone()))
      .children(declaration.collects_data.then(|| div().child(if data.is_empty() { tr(l, "addCollectedDataTypes") } else { data.clone() })))
      .children(declaration.uses_tracking.then(|| div().child(tr(l, "trackingDeclared"))));
    let dialog_title = title.clone();
    let button = Button::new("privacy-preview-details").small().outline().label(tr(l, "previewDetails")).on_click(move |_, window, cx| {
      let (title, details, data) = (dialog_title.clone(), details.clone(), data.clone());
      window.open_dialog(cx, move |dialog, _, _| {
        dialog.title(title.clone()).child(v_flex().gap_3().child(details.clone()).children((!data.is_empty()).then(|| div().child(data.clone()))))
      });
    });
    card(title, None, Some(button.into_any_element()), body, &c)
  }
}
