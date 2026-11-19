//! Progress and result notifications for manual App Store Connect refreshes.

use super::*;

#[derive(Clone, Copy)]
pub(crate) enum Refresh {
  AppDetails,
  Distribution,
  ProductMedia,
  Reviews,
}

impl Refresh {
  fn key(self) -> &'static str {
    match self {
      Self::AppDetails => "refreshAppDetails",
      Self::Distribution => "refreshDistributionData",
      Self::ProductMedia => "refreshProductMedia",
      Self::Reviews => "refreshCustomerReviews",
    }
  }

  fn id(self, app: &str) -> SharedString {
    format!("{app}-{}", self.key()).into()
  }

  pub(crate) fn start(self, app: &str, language: Language, window: &mut Window, cx: &mut App) {
    let message = trf(language, "refreshStarted", &[&tr(language, self.key())]);
    window.push_notification(Notification::info(message).id1::<Self>(self.id(app)), cx);
  }

  pub(crate) fn finish<T>(self, app: &str, language: Language, result: &Result<T, appforge_appstore::Error>, window: &mut Window, cx: &mut App) {
    let subject = tr(language, self.key());
    let notification = match result {
      Ok(_) => Notification::success(trf(language, "refreshSucceeded", &[&subject])),
      Err(error) => Notification::error(trf(language, "refreshFailed", &[&subject, &error.to_string()])),
    };
    window.push_notification(notification.id1::<Self>(self.id(app)), cx);
  }
}
