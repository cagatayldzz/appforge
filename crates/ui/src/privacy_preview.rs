//! The local preview of the App Privacy label: what the product page would say about the answers.

use crate::{i18n::tr, theme::colors, widgets::card, workspace::Workspace};
use appforge_storage::PrivacyDeclaration;
use gpui_kit::{
  component::{button::Button, v_flex, Sizable as _, WindowExt as _},
  *,
};

impl Workspace {
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
