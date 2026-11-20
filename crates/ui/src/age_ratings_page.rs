//! The Age Ratings section of App Information: the ratings Apple returns and where to change them.

use crate::{
  i18n::{tr, trf},
  product_page::{info_field, store_info, store_link},
  theme::colors,
  widgets::section,
  workspace::Workspace,
};
use appforge_appstore::App as StoreApp;
use gpui_kit::{
  component::{alert::Alert, button::Button, h_flex, v_flex, Disableable as _, Icon, IconName, Sizable as _},
  *,
};

impl Workspace {
  pub(crate) fn render_age_ratings_section(&self, app: &StoreApp, cx: &mut Context<Self>) -> impl IntoElement {
    let l = self.settings.language;
    let c = colors(cx);
    let info = self.ready_snapshot().map(|snapshot| &snapshot.general);
    let field = |key: &'static str, value: &str| info_field(l, &c, key, value);
    let mut ratings = h_flex().gap_3().flex_wrap();
    if let Some(info) = info {
      ratings = ratings.children(
        info
          .ratings
          .iter()
          .map(|(label, value)| field(if label == "Global" { "globalRating" } else { "regionalRating" }, &format!("{label} · {value}"))),
      );
    }
    let mut territories = v_flex().gap_2();
    if let Some(info) = info {
      if let Some(error) = &info.territory_ratings_error {
        territories = territories.child(Alert::warning("territory-ratings-error", SharedString::from(error.clone())));
      } else if !info.territory_ratings.is_empty() {
        let mut groups = std::collections::BTreeMap::<&str, usize>::new();
        for (_, rating) in &info.territory_ratings {
          *groups.entry(rating).or_default() += 1;
        }
        territories = territories.child(h_flex().gap_3().flex_wrap().children(
          groups.into_iter().map(|(rating, count)| field("regionalRating", &trf(l, "ageRatingTerritoryCount", &[rating, &count.to_string()]))),
        ));
      }
    }
    if info.is_none_or(|info| info.ratings.is_empty() && info.territory_ratings.is_empty() && info.territory_ratings_error.is_none()) {
      ratings = ratings.child(tr(l, "noAgeRatings"));
    }
    section(
      tr(l, "ageRatings"),
      v_flex().gap_3().child(ratings).child(territories).children(info.map(|info| field("ageSuitabilityUrl", &info.age_suitability_url))).child(
        h_flex()
          .gap_2()
          .flex_wrap()
          .children(info.is_some_and(|info| info.age_declaration.is_some()).then(|| {
            Button::new("edit-age-ratings")
              .label(tr(l, "editAgeRatings"))
              .icon(Icon::new(IconName::Settings))
              .small()
              .outline()
              .disabled(self.store_write_busy || self.saving)
              .on_click(cx.listener(|this, _, window, cx| this.open_age_rating_editor(window, cx)))
          }))
          .child(store_link("manage-age-ratings", tr(l, "manageAgeRatings"), store_info(app))),
      ),
    )
  }
}
