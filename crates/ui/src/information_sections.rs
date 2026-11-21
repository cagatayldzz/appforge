//! The parts of App Information that only App Store Connect offers, laid out as it does: encryption documentation, server
//! notifications, the shared secret and the additional links. Each opens the page in the browser.

use crate::{
  i18n::tr,
  product_page::store_info,
  theme::{colors, Colors},
  widgets::{page_section, text_link},
  workspace::Workspace,
};
use appforge_appstore::App as StoreApp;
use gpui_kit::{
  component::{
    button::{Button, ButtonVariants as _},
    h_flex, v_flex, Sizable as _,
  },
  *,
};

const ENCRYPTION_HELP_URL: &str = "https://developer.apple.com/documentation/security/complying_with_encryption_export_regulations";
const NOTIFICATIONS_HELP_URL: &str =
  "https://developer.apple.com/help/app-store-connect/configure-in-app-purchase-settings/enter-server-urls-for-app-store-server-notifications";

impl Workspace {
  /// A text followed by its "Learn More" link, wrapping like one paragraph.
  fn paragraph_with_link(&self, id: &'static str, text: &'static str, url: &'static str, c: &Colors) -> Div {
    let l = self.settings.language;
    h_flex().flex_wrap().gap_x_1().child(div().child(tr(l, text))).child(text_link(id, tr(l, "learnMore"), url, c))
  }

  /// A grey panel with one centered text link, for something that is not set up yet.
  fn setup_panel(&self, id: &'static str, label: &'static str, url: String, c: &Colors) -> Div {
    let l = self.settings.language;
    v_flex()
      .flex_1()
      .min_w_0()
      .gap_2()
      .child(div().font_weight(FontWeight::MEDIUM).child(tr(l, label)))
      .child(div().flex().items_center().justify_center().h_32().rounded_md().bg(c.surface).child(text_link(id, tr(l, "setUpUrl"), url, c)))
  }

  pub(crate) fn render_information_extras(&self, app: &StoreApp, cx: &mut Context<Self>) -> Vec<AnyElement> {
    let l = self.settings.language;
    let c = colors(cx);
    let info = store_info(app);
    let upload = info.clone();
    let encryption = page_section(
      tr(l, "appEncryptionDocumentation"),
      v_flex()
        .gap_3()
        .child(self.paragraph_with_link("encryption-learn-more", "appEncryptionDocumentationHelp", ENCRYPTION_HELP_URL, &c))
        .child(div().child(tr(l, "encryptionRequired")))
        .child(v_flex().gap_1().pl_4().child(tr(l, "encryptionProprietaryItem")).child(tr(l, "encryptionStandardItem")))
        .child(
          v_flex()
            .items_center()
            .gap_3()
            .py_6()
            .rounded_md()
            .bg(c.surface)
            .child(tr(l, "encryptionProvideBefore"))
            .child(Button::new("upload-encryption-docs").label(tr(l, "upload")).primary().small().on_click(move |_, _, cx| cx.open_url(&upload))),
        ),
    );
    let notifications = page_section(
      tr(l, "serverNotifications"),
      v_flex().gap_3().child(self.paragraph_with_link("notifications-learn-more", "serverNotificationsHelp", NOTIFICATIONS_HELP_URL, &c)).child(
        h_flex().gap_6().child(self.setup_panel("setup-production-url", "productionServerUrl", info.clone(), &c)).child(self.setup_panel(
          "setup-sandbox-url",
          "sandboxServerUrl",
          info.clone(),
          &c,
        )),
      ),
    );
    let secret = page_section(
      tr(l, "sharedSecret"),
      v_flex().gap_3().child(div().child(tr(l, "sharedSecretHelp"))).child(text_link("manage-shared-secret", tr(l, "manage"), info.clone(), &c)),
    );
    let additional = page_section(
      tr(l, "additionalInformation"),
      v_flex()
        .gap_3()
        .child(text_link("public-app-store", tr(l, "viewOnAppStore"), format!("https://apps.apple.com/app/id{}", app.id), &c))
        .child(text_link("manage-app-access", tr(l, "manageAppAccess"), "https://appstoreconnect.apple.com/access/users", &c))
        .child(text_link("remove-app", tr(l, "removeApp"), info, &c)),
    );
    [encryption, notifications, secret, additional].map(IntoElement::into_any_element).into()
  }
}
