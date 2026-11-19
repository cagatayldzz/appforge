//! The page of all apps: one card per app in a grid of three columns.

use crate::{
  i18n::{tr, trf},
  theme::{colors, FONT_SIZE_SM},
  widgets::{app_avatar, state_badge},
  workspace::{Load, Workspace},
};
use appforge_appstore::App;
use gpui_kit::{
  component::{alert::Alert, h_flex, scroll::ScrollableElement as _, skeleton::Skeleton, tag::Tag, v_flex, Sizable as _},
  prelude::FluentBuilder as _,
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
    let loading = matches!(self.apps, Load::Loading);
    let heading = h_flex()
      .w_full()
      .justify_between()
      .items_start()
      .child(v_flex().gap_1().child(div().text_xl().font_weight(FontWeight::SEMIBOLD).child(tr(l, "Apps"))).child(if loading {
        Skeleton::new().h_3().w_20().into_any_element()
      } else {
        div().text_size(FONT_SIZE_SM).text_color(c.text_secondary).child(trf(l, "{} apps", &[&apps.len().to_string()])).into_any_element()
      }))
      .child(self.reload_button(cx));
    let skeletons = (0..COLUMNS * 2).map(|n| skeleton_card(n, &c));
    let failure = match &self.apps {
      Load::Failed(message) => Some(Alert::error("apps-error", message.clone())),
      _ => None,
    };
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
            h_flex().gap_3().items_center().child(app_avatar(&app.id, &app.name, true, false, self.icons.get(&app.id), cx).size_10()).child(
              v_flex()
                .flex_1()
                .min_w_0()
                .child(div().truncate().font_weight(FontWeight::SEMIBOLD).child(app.name.clone()))
                .child(div().truncate().text_size(FONT_SIZE_SM).text_color(c.text_secondary).child(app.bundle_id.clone())),
            ),
          )
          .child(h_flex().gap_2().items_center().justify_between().child(Tag::secondary().small().child(app.primary_locale.clone())).when_some(
            self.versions.get(&app.id),
            |row, version| {
              row.child(
                h_flex()
                  .gap_2()
                  .items_center()
                  .text_size(FONT_SIZE_SM)
                  .child(div().font_weight(FontWeight::MEDIUM).child(format!("v{}", version.version_string)))
                  .child(state_badge(&version.state, &c)),
              )
            },
          ))
          .on_click(cx.listener(move |this, _, window, cx| this.select_app(id.clone(), window, cx)))
          .into_any_element()
      })
      .collect();
    v_flex().id("apps-grid").flex_1().min_h_0().overflow_y_scrollbar().child(
      h_flex().w_full().justify_center().child(
        v_flex()
          .w_full()
          .max_w(px(GRID_MAX_WIDTH))
          .px_6()
          .py_6()
          .gap_4()
          .child(heading)
          .children(failure)
          .child(div().grid().grid_cols(COLUMNS).gap_4().children(cards).children(skeletons.filter(|_| loading))),
      ),
    )
  }
}

/// A card-shaped placeholder shown while the apps load: an icon, two lines of text and a footer line.
fn skeleton_card(n: u16, c: &crate::theme::Colors) -> impl IntoElement {
  v_flex()
    .id(SharedString::from(format!("app-skeleton-{n}")))
    .gap_3()
    .p_4()
    .rounded_lg()
    .border_1()
    .border_color(c.border)
    .child(
      h_flex()
        .gap_3()
        .items_center()
        .child(Skeleton::new().size_10().rounded_lg().flex_none())
        .child(v_flex().flex_1().gap_2().child(Skeleton::new().h_4().w_32()).child(Skeleton::new().h_3().w_40())),
    )
    .child(h_flex().justify_between().child(Skeleton::new().h_4().w_12()).child(Skeleton::new().h_4().w_24()))
}
