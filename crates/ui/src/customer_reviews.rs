//! The Ratings and Reviews page: summary, filters, review cards and developer replies.

use crate::{
  i18n::{tr, trf},
  theme::{colors, FONT_SIZE_SM},
  workspace::{Load, Workspace},
};
mod cards;

use appforge_appstore::CustomerReview;
use gpui_kit::{
  component::{
    alert::Alert,
    button::Button,
    h_flex,
    menu::{DropdownMenu as _, PopupMenuItem},
    spinner::Spinner,
    v_flex, Icon, IconName, Sizable as _,
  },
  *,
};

/// Which reviews the page lists, by whether the developer has answered them.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum ReviewStatus {
  #[default]
  All,
  Unanswered,
  Answered,
}

impl ReviewStatus {
  const ALL: [ReviewStatus; 3] = [Self::All, Self::Unanswered, Self::Answered];

  fn label(self) -> &'static str {
    match self {
      Self::All => "allReviewsFilter",
      Self::Unanswered => "unanswered",
      Self::Answered => "answered",
    }
  }

  fn matches(self, review: &CustomerReview) -> bool {
    match self {
      Self::All => true,
      Self::Unanswered => review.response.is_none(),
      Self::Answered => review.response.is_some(),
    }
  }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum ReviewSort {
  #[default]
  Recent,
  Highest,
  Lowest,
}

impl ReviewSort {
  fn label(self) -> &'static str {
    match self {
      Self::Recent => "recentReviews",
      Self::Highest => "highestRatedReviews",
      Self::Lowest => "lowestRatedReviews",
    }
  }
}

fn filter_menu<T: Clone + 'static>(
  id: &'static str,
  label: SharedString,
  choices: Vec<(SharedString, T, bool)>,
  cx: &Context<Workspace>,
  select: impl Fn(&mut Workspace, T, &mut Window, &mut Context<Workspace>) + Copy + 'static,
) -> impl IntoElement {
  let workspace = cx.weak_entity();
  Button::new(id).label(label).icon(Icon::new(IconName::ChevronDown)).outline().small().dropdown_menu(move |menu, _, _| {
    let menu = menu.scrollable(choices.len() > 10);
    choices.iter().fold(menu, |menu, (label, value, selected)| {
      let (workspace, value) = (workspace.clone(), value.clone());
      menu.item(PopupMenuItem::new(label.clone()).checked(*selected).on_click(move |_, window, cx| {
        workspace
          .update(cx, |this, cx| {
            select(this, value.clone(), window, cx);
            cx.notify();
          })
          .ok();
      }))
    })
  })
}

impl Workspace {
  pub(crate) fn load_customer_reviews(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let (Some(client), Some(app)) = (self.client.clone(), self.selected.clone()) else { return };
    let Some(data) = self.app_data.get_mut(&app) else { return };
    if data.reviews_task.is_some() {
      return;
    }
    if !matches!(data.reviews, Load::Ready(_)) {
      data.reviews = Load::Loading;
    }
    let target = app.clone();
    let version = data.review_version.clone();
    let needs_versions = data.review_versions.is_none();
    data.reviews_task = Some(cx.spawn_in(window, async move |this, cx| {
      let (result, versions) = cx
        .background_spawn(async move {
          let reviews = match version {
            Some(id) => client.version_customer_reviews(&id),
            None => client.customer_reviews(&target),
          };
          let versions = needs_versions.then(|| client.review_versions(&target));
          (reviews, versions)
        })
        .await;
      this
        .update_in(cx, |this, window, cx| {
          if this.require_apple_sign_in(&result, window, cx) {
            return;
          }
          let Some(data) = this.app_data.get_mut(&app) else { return };
          data.reviews_task = None;
          let active = this.selected.as_ref() == Some(&app);
          match versions {
            Some(Ok(versions)) => data.review_versions = Some(versions),
            Some(Err(error)) if active && result.is_ok() => Self::warning(window, cx, error.to_string()),
            _ => {}
          }
          match result {
            Ok(reviews) => {
              data.review_territories.extend(reviews.iter().filter(|review| !review.territory.is_empty()).map(|review| review.territory.clone()));
              data.reviews = Load::Ready(reviews);
            }
            Err(error) => {
              if !matches!(data.reviews, Load::Ready(_)) {
                data.reviews = Load::Failed(error.to_string().into());
              }
              if active {
                Self::error(window, cx, error.to_string());
              }
            }
          }
          if active {
            cx.notify();
          }
        })
        .ok();
    }));
    cx.notify();
  }

  pub(crate) fn render_customer_reviews(&self, cx: &mut Context<Self>) -> AnyElement {
    let l = self.settings.language;
    let state = self.active_data().map(|data| &data.reviews);
    let reviews = match state {
      Some(Load::Ready(reviews)) => reviews.as_slice(),
      _ => &[],
    };
    let pending = match state {
      Some(Load::Ready(_)) => None,
      Some(Load::Failed(message)) => Some(
        v_flex()
          .gap_3()
          .child(Alert::error("customer-reviews-error", message.clone()))
          .child(
            h_flex().child(
              Button::new("retry-customer-reviews")
                .label(tr(l, "tryAgain"))
                .outline()
                .on_click(cx.listener(|this, _, window, cx| this.load_customer_reviews(window, cx))),
            ),
          )
          .into_any_element(),
      ),
      _ => Some(h_flex().gap_2().child(Spinner::new()).child(tr(l, "loadingFromAppStore")).into_any_element()),
    };
    self.reviews_page(reviews, pending, cx).into_any_element()
  }

  /// The summary, the filters and the reviews that pass them.
  fn reviews_page(&self, reviews: &[CustomerReview], pending: Option<AnyElement>, cx: &mut Context<Self>) -> Div {
    let l = self.settings.language;
    let c = colors(cx);
    let (status, rating, sort) = (self.review_status, self.review_rating, self.review_sort);
    let territory = self.review_territory.as_deref();
    let scoped: Vec<_> = reviews.iter().filter(|review| territory.is_none_or(|territory| territory == review.territory)).collect();
    let mut visible: Vec<_> = scoped.iter().copied().filter(|review| status.matches(review) && rating.is_none_or(|n| review.rating == n)).collect();
    match sort {
      ReviewSort::Recent => visible.sort_by(|left, right| right.created.cmp(&left.created)),
      ReviewSort::Highest => visible.sort_by_key(|review| std::cmp::Reverse(review.rating)),
      ReviewSort::Lowest => visible.sort_by_key(|review| review.rating),
    }
    let filtered = status != ReviewStatus::All || rating.is_some() || territory.is_some();
    let list = if let Some(pending) = pending {
      pending
    } else if visible.is_empty() {
      v_flex()
        .gap_3()
        .items_start()
        .child(div().text_color(c.text_secondary).child(tr(l, if filtered { "noReviewsMatchThese" } else { "noCustomerReviews" })))
        .children(filtered.then(|| {
          Button::new("clear-review-filters").label(tr(l, "clearFilters")).outline().small().on_click(cx.listener(|this, _, _, cx| {
            this.review_status = ReviewStatus::All;
            this.review_rating = None;
            this.review_territory = None;
            cx.notify();
          }))
        }))
        .into_any_element()
    } else {
      h_flex()
        .w_full()
        .flex_wrap()
        .gap_3()
        .items_start()
        .children(visible.iter().map(|review| div().flex_1().flex_basis(relative(0.47)).min_w_64().child(self.review_card(review, cx))))
        .into_any_element()
    };
    let data = self.active_data();
    let current_version = data.and_then(|data| data.review_version.as_ref());
    let versions = data.and_then(|data| data.review_versions.as_deref()).unwrap_or_default();
    let version_menu = filter_menu(
      "review-version-filter",
      versions.iter().find(|(id, _)| Some(id) == current_version).map_or(tr(l, "allReviewVersions"), |(_, label)| format!("iOS {label}").into()),
      std::iter::once((tr(l, "allReviewVersions"), None, current_version.is_none()))
        .chain(versions.iter().map(|(id, label)| (format!("iOS {label}").into(), Some(id.clone()), Some(id) == current_version)))
        .collect(),
      cx,
      |this, value, window, cx| {
        let Some(app) = this.selected.as_ref().and_then(|id| this.app_data.get_mut(id)) else { return };
        if app.review_version != value {
          app.review_version = value;
          app.reviews_task = None;
          app.reviews = Load::Idle;
          this.load_customer_reviews(window, cx);
        }
      },
    );
    let territory_menu = filter_menu(
      "review-territory-filter",
      territory.map_or(tr(l, "allReviewTerritories"), |code| code.to_owned().into()),
      std::iter::once((tr(l, "allReviewTerritories"), None, territory.is_none()))
        .chain(
          data.into_iter().flat_map(|data| &data.review_territories).map(|code| (code.clone().into(), Some(code.clone()), territory == Some(code))),
        )
        .collect(),
      cx,
      |this, value, _, _| this.review_territory = value,
    );
    let ratings_menu = filter_menu(
      "review-rating-filter",
      rating.map_or(tr(l, "allReviewRatings"), |rating| format!("{rating} ★").into()),
      std::iter::once((tr(l, "allReviewRatings"), None, rating.is_none()))
        .chain((1..=5).rev().map(|n| (format!("{n} ★").into(), Some(n), rating == Some(n))))
        .collect(),
      cx,
      |this, value, _, _| this.review_rating = value,
    );
    let status_menu = filter_menu(
      "review-status-filter",
      tr(l, status.label()),
      ReviewStatus::ALL.into_iter().map(|item| (tr(l, item.label()), item, status == item)).collect(),
      cx,
      |this, value, _, _| this.review_status = value,
    );
    let sort_menu = filter_menu(
      "review-sort",
      trf(l, "reviewsWithCount", &[&tr(l, sort.label()), &visible.len().to_string()]),
      [ReviewSort::Recent, ReviewSort::Highest, ReviewSort::Lowest].into_iter().map(|item| (tr(l, item.label()), item, sort == item)).collect(),
      cx,
      |this, value, _, _| this.review_sort = value,
    );
    v_flex()
      .gap_4()
      .child(
        h_flex()
          .justify_between()
          .gap_2()
          .flex_wrap()
          .child(div().font_weight(FontWeight::SEMIBOLD).child(tr(l, "writtenReviewRatings")))
          .child(territory_menu),
      )
      .child(self.reviews_summary(&scoped, cx))
      .child(
        h_flex()
          .mt_3()
          .w_full()
          .justify_between()
          .gap_2()
          .flex_wrap()
          .child(sort_menu)
          .child(h_flex().gap_1().flex_wrap().child(version_menu).child(ratings_menu).child(status_menu)),
      )
      .child(list)
  }

  fn reviews_summary(&self, reviews: &[&CustomerReview], cx: &Context<Self>) -> Div {
    let l = self.settings.language;
    let c = colors(cx);
    let mut counts = [0usize; 5];
    for review in reviews {
      if (1..=5).contains(&review.rating) {
        counts[review.rating as usize - 1] += 1;
      }
    }
    let total: usize = counts.iter().sum();
    let average = if total == 0 {
      "—".to_owned()
    } else {
      let sum: usize = counts.iter().enumerate().map(|(index, count)| (index + 1) * count).sum();
      format!("{:.1}", sum as f64 / total as f64)
    };
    let rows = counts.into_iter().enumerate().rev().map(|(index, count)| {
      let rating = index as u64 + 1;
      h_flex()
        .gap_3()
        .items_center()
        .w_full()
        .child(div().w_16().flex_none().text_right().whitespace_nowrap().text_color(c.warning).child("★".repeat(index + 1)))
        .child(
          div()
            .flex_1()
            .h_1()
            .rounded_full()
            .bg(c.border)
            .child(div().h_full().rounded_full().bg(c.warning).w(relative(count as f32 / total.max(1) as f32))),
        )
        .child(div().w_8().text_right().text_size(FONT_SIZE_SM).text_color(c.text_secondary).child(count.to_string()))
        .id(("review-breakdown", rating))
    });
    h_flex()
      .gap_3()
      .flex_wrap()
      .w_full()
      .child(
        v_flex()
          .flex_1()
          .flex_basis(relative(0.30))
          .min_w_40()
          .p_4()
          .gap_1()
          .rounded_lg()
          .bg(c.rail)
          .child(div().text_3xl().font_weight(FontWeight::SEMIBOLD).child(average))
          .child(div().text_color(c.text_secondary).child(tr(l, "outOfFive"))),
      )
      .child(
        v_flex()
          .flex_1()
          .flex_basis(relative(0.60))
          .min_w_64()
          .p_4()
          .gap_1()
          .rounded_lg()
          .bg(c.rail)
          .children(rows)
          .child(div().text_right().text_size(FONT_SIZE_SM).text_color(c.text_secondary).child(trf(l, "reviews", &[&total.to_string()]))),
      )
  }
}
