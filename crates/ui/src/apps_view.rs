//! The page of all apps: one card per app in a grid of three columns.

use crate::{
  i18n::{tr, trf},
  theme::{colors, FONT_SIZE_SM},
  widgets::app_avatar,
  workspace::{Load, Workspace},
};
use appforge_appstore::App;
use gpui_kit::{
  component::{h_flex, scroll::ScrollableElement as _, tag::Tag, v_flex, Sizable as _},
  *,
};

/// The widest the grid grows; cards get too wide on a big window otherwise.
const GRID_MAX_WIDTH: f32 = 1040.;
/// How many cards sit side by side.
const COLUMNS: u16 = 3;

impl Workspace {
  pub(crate) fn render_apps(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
    let c = colors(cx);
    let l = self.settings.language;
    let apps: Vec<App> = match &self.apps {
      Load::Ready(apps) => apps.clone(),
      _ => Vec::new(),
    };
    let heading = v_flex()
      .gap_1()
      .child(div().text_xl().font_weight(FontWeight::SEMIBOLD).child(tr(l, "Apps")))
      .child(div().text_size(FONT_SIZE_SM).text_color(c.text_secondary).child(trf(l, "{} apps", &[&apps.len().to_string()])));
    let cards: Vec<AnyElement> = apps
      .into_iter()
      .map(|app| {
        let id = app.id.clone();
        v_flex()
          .id(SharedString::from(format!("app-card-{}", app.id)))
          .gap_3()
          .p_4()
          .rounded_lg()
          .border_1()
          .border_color(c.border)
          .cursor_pointer()
          .hover(move |card| card.bg(c.hover).border_color(c.control_border))
          .child(
            h_flex().gap_3().items_center().child(app_avatar(&app.id, &app.name, true).size_10()).child(
              v_flex()
                .flex_1()
                .min_w_0()
                .child(div().truncate().font_weight(FontWeight::SEMIBOLD).child(app.name.clone()))
                .child(div().truncate().text_size(FONT_SIZE_SM).text_color(c.text_secondary).child(app.bundle_id.clone())),
            ),
          )
          .child(h_flex().gap_2().child(Tag::secondary().small().child(app.primary_locale.clone())))
          .on_click(cx.listener(move |this, _, window, cx| this.select_app(id.clone(), window, cx)))
          .into_any_element()
      })
      .collect();
    v_flex().id("apps-grid").flex_1().min_h_0().overflow_y_scrollbar().child(h_flex().w_full().justify_center().child(
      v_flex().w_full().max_w(px(GRID_MAX_WIDTH)).px_6().py_6().gap_4().child(heading).child(div().grid().grid_cols(COLUMNS).gap_4().children(cards)),
    ))
  }
}
