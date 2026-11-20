//! Pages App Store Connect owns because the API has no endpoint for them, and the rejection notice of the Distribution page.

use crate::{
  i18n::tr,
  product_page::store_link,
  theme::colors,
  widgets::{section, state_badge},
  workspace::{Load, Workspace},
};
use appforge_appstore::{App as StoreApp, AGE_LEVEL_KEYS};
use gpui_kit::{
  component::{alert::Alert, h_flex, v_flex},
  *,
};

impl Workspace {
  /// A short explanation and a link to the page in App Store Connect.
  pub(crate) fn render_store_only(&self, app: &StoreApp, page: &'static str, help: &'static str, cx: &mut Context<Self>) -> impl IntoElement {
    let _ = cx;
    let l = self.settings.language;
    let url = format!("https://appstoreconnect.apple.com/apps/{}/distribution/{page}", app.id);
    v_flex().gap_4().child(Alert::info(SharedString::from(format!("store-only-{page}")), tr(l, help))).child(h_flex().child(store_link(
      page,
      tr(l, "openInAppStoreConnect"),
      url,
    )))
  }

  /// App Compliance: what App Store Connect lists on its page. The export compliance declarations come from the API; whether the
  /// medical device and Vietnam game license declarations apply follows from the categories and age rating answers. The other
  /// declarations have no API, so each shows Apple's explanation and a link to where it is answered.
  pub(crate) fn render_compliance(&self, app: &StoreApp, cx: &mut Context<Self>) -> impl IntoElement {
    let l = self.settings.language;
    let c = colors(cx);
    let url = format!("https://appstoreconnect.apple.com/apps/{}/distribution/compliance", app.id);
    let general = self.ready_snapshot().map(|snapshot| &snapshot.general);
    let link = |id: &'static str| h_flex().child(store_link(id, tr(l, "openInAppStoreConnect"), url.clone()));
    let note = |text: SharedString| div().text_sm().text_color(c.text_secondary).child(text);
    let item = |id: &'static str, title: &'static str, help: &'static str, status: Option<SharedString>| {
      section(tr(l, title), v_flex().gap_3().child(div().text_sm().child(tr(l, help))).children(status.map(&note)).child(link(id)))
    };

    // Export compliance: the declarations the API returns.
    let encryption = match general.and_then(|general| general.encryption.as_ref()) {
      None => note(tr(l, "complianceUnavailable")).into_any_element(),
      Some(items) if items.is_empty() => note(tr(l, "complianceNoEncryption")).into_any_element(),
      Some(items) => v_flex()
        .gap_2()
        .children(items.iter().enumerate().map(|(index, declaration)| {
          let mut facts = Vec::new();
          for (flag, key) in [
            (declaration.exempt, "encryptionExempt"),
            (declaration.proprietary, "encryptionProprietary"),
            (declaration.third_party, "encryptionThirdParty"),
            (declaration.french_store, "encryptionFrench"),
          ] {
            if flag {
              facts.push(tr(l, key).to_string());
            }
          }
          h_flex()
            .id(SharedString::from(format!("encryption-{index}-{}", declaration.created)))
            .gap_3()
            .items_center()
            .flex_wrap()
            .child(state_badge(&declaration.state, &c))
            .child(note(SharedString::from(declaration.created.chars().take(10).collect::<String>())))
            .child(note(SharedString::from(facts.join(" · "))))
        }))
        .into_any_element(),
    };
    let export = section(tr(l, "complianceExport"), v_flex().gap_3().child(div().text_sm().child(tr(l, "complianceExportHelp"))).child(encryption));

    // Medical devices and the Vietnam license: whether they apply follows from what the app already declares.
    let categories: Vec<&str> = general.map(|general| general.categories.iter().map(String::as_str).collect()).unwrap_or_default();
    let medical_category = categories.iter().any(|category| matches!(*category, "MEDICAL" | "HEALTH_AND_FITNESS"));
    let medical_answer = general
      .and_then(|general| general.age_declaration.as_ref())
      .zip(AGE_LEVEL_KEYS.iter().position(|key| *key == "medicalOrTreatmentInformation"))
      .is_some_and(|(declaration, index)| declaration.levels[index] == "FREQUENT_OR_INTENSE");
    let medical =
      general.map(|_| tr(l, if medical_category || medical_answer { "complianceMedicalApplies" } else { "complianceMedicalNotApplicable" }));
    let game = categories.first().is_some_and(|category| *category == "GAMES");
    let vietnam = general.map(|_| tr(l, if game { "complianceVietnamGame" } else { "complianceVietnamNotGame" }));

    v_flex()
      .gap_6()
      .child(Alert::info("store-only-compliance", tr(l, "appComplianceHelp")))
      .child(export)
      .child(item("compliance-medical", "complianceMedical", "complianceMedicalHelp", medical))
      .child(item("compliance-vietnam", "complianceVietnam", "complianceVietnamHelp", vietnam))
      .child(item("compliance-security", "complianceSecurityAssessment", "complianceSecurityAssessmentHelp", Some(tr(l, "complianceNoApi"))))
      .child(item("compliance-ai", "complianceAiContent", "complianceAiContentHelp", Some(tr(l, "complianceNoApi"))))
      .child(item("compliance-dsa", "complianceDsa", "complianceDsaHelp", Some(tr(l, "complianceNoApi"))))
      .child(item("compliance-icp", "complianceIcp", "complianceIcpHelp", Some(tr(l, "complianceNoApi"))))
  }

  /// "This item requires your attention", as App Store Connect shows it when Apple rejected what was submitted.
  /// The API does not return Apple's reasons; the link opens the submission in App Store Connect.
  pub(crate) fn rejection_notice(&self, app: &StoreApp) -> Option<AnyElement> {
    let l = self.settings.language;
    let Some(Load::Ready(data)) = self.active_data().map(|data| &data.distribution) else { return None };
    let rejected =
      data.submissions.iter().filter(|item| item.includes_version && item.state == "UNRESOLVED_ISSUES").max_by(|a, b| a.date.cmp(&b.date))?;
    let url = format!("https://appstoreconnect.apple.com/apps/{}/distribution/reviewsubmissions/details/{}", app.id, rejected.id);
    Some(
      v_flex()
        .gap_2()
        .child(Alert::error("submission-rejected", tr(l, "submissionRejected")).title(tr(l, "requiresAttention")))
        .child(h_flex().child(store_link("view-submission", tr(l, "viewSubmission"), url)))
        .into_any_element(),
    )
  }
}
