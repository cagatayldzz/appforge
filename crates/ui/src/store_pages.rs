//! Pages App Store Connect owns because the API has no endpoint for them, and the rejection notice of the Distribution page.

use crate::{
  i18n::tr,
  product_page::store_link,
  workspace::{Load, Workspace},
};
use appforge_appstore::App as StoreApp;
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
