//! The About page of Settings: the running version, app updates and where the data lives.

use crate::{
  i18n::{tr, trf},
  theme::{colors, FONT_SIZE_SM},
  widgets::card,
  workspace::Workspace,
};
use gpui_kit::{
  component::{button::Button, h_flex, v_flex, Icon, IconName},
  *,
};

impl Workspace {
  /// A card for each topic, stacked in one column: the version, the updates, the data folder.
  pub(crate) fn about_body(&mut self, cx: &mut Context<Self>) -> AnyElement {
    let l = self.settings.language;
    let c = colors(cx);
    let data_dir = self.data_dir.clone();
    let version = h_flex().w_full().gap_3().items_center().child(div().font_weight(FontWeight::SEMIBOLD).text_lg().child("AppForge")).child(
      div().px_2().py_0p5().rounded_full().border_1().border_color(c.border).text_size(FONT_SIZE_SM).text_color(c.text_secondary).child(trf(
        l,
        "versionNumber",
        &[env!("CARGO_PKG_VERSION")],
      )),
    );
    let folder = h_flex()
      .w_full()
      .gap_3()
      .flex_wrap()
      .items_center()
      .justify_between()
      .child(
        v_flex()
          .flex_1()
          .flex_basis(rems(12.))
          .min_w_0()
          .gap_0p5()
          .child(div().font_weight(FontWeight::MEDIUM).child(tr(l, "dataFolder")))
          .child(div().text_size(FONT_SIZE_SM).text_color(c.text_secondary).child(tr(l, "settingsAndNotesAre"))),
      )
      .child(
        Button::new("settings-data-folder")
          .icon(Icon::new(IconName::FolderOpen))
          .label(tr(l, "showDataFolder"))
          .outline()
          .on_click(move |_, _, cx| cx.reveal_path(&data_dir)),
      );
    let boxed = |child: Div| v_flex().w_full().p_4().rounded_lg().border_1().border_color(c.border).child(child);
    let updates = self.render_updates(cx);
    v_flex().gap_4().child(boxed(version)).child(card(tr(l, "appUpdates"), None, None, updates, &c)).child(boxed(folder)).into_any_element()
  }
}
