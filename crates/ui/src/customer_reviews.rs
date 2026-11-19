//! The Ratings and Reviews page: summary, filters, review cards and developer replies.

use crate::{
  distribution::Change,
  forms::{read_area, set_area},
  i18n::{tr, trf},
  theme::{colors, Colors, FONT_SIZE_SM},
  widgets::{area_field, state_badge},
  workspace::{Load, Workspace},
};
use appforge_appstore::CustomerReview;
use gpui_kit::{
  component::{
    alert::Alert,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::TextareaState,
    spinner::Spinner,
    v_flex, Disableable as _, Selectable as _, Sizable as _, WindowExt as _,
  },
  prelude::FluentBuilder as _,
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
      Self::All => "all",
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

/// The stars of a rating: filled ones in the warning color, the rest faint.
fn stars(rating: u64, c: &Colors) -> Div {
  h_flex().gap_0p5().text_color(c.warning).children((1..=5u64).map(|n| div().text_color(if n <= rating { c.warning } else { c.border }).child("★")))
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
    data.reviews_task = Some(cx.spawn_in(window, async move |this, cx| {
      let result = cx.background_spawn(async move { client.customer_reviews(&target) }).await;
      this
        .update_in(cx, |this, window, cx| {
          if this.require_apple_sign_in(&result, window, cx) {
            return;
          }
          let Some(data) = this.app_data.get_mut(&app) else { return };
          data.reviews_task = None;
          let active = this.selected.as_ref() == Some(&app);
          match result {
            Ok(reviews) => data.reviews = Load::Ready(reviews),
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
    match self.active_data().map(|data| &data.reviews) {
      Some(Load::Ready(reviews)) if reviews.is_empty() => div().child(tr(l, "noCustomerReviews")).into_any_element(),
      Some(Load::Ready(reviews)) => self.reviews_page(reviews, cx).into_any_element(),
      Some(Load::Failed(message)) => v_flex()
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
      _ => h_flex().gap_2().child(Spinner::new()).child(tr(l, "loadingFromAppStore")).into_any_element(),
    }
  }

  /// The summary, the filters and the reviews that pass them.
  fn reviews_page(&self, reviews: &[CustomerReview], cx: &mut Context<Self>) -> Div {
    let l = self.settings.language;
    let c = colors(cx);
    let (status, rating) = (self.review_status, self.review_rating);
    let visible: Vec<&CustomerReview> = reviews.iter().filter(|review| status.matches(review) && rating.is_none_or(|n| review.rating == n)).collect();
    let filtered = status != ReviewStatus::All || rating.is_some();
    let list = if visible.is_empty() {
      v_flex()
        .gap_3()
        .items_start()
        .child(div().text_color(c.text_secondary).child(tr(l, "noReviewsMatchThese")))
        .child(Button::new("clear-review-filters").label(tr(l, "clearFilters")).outline().small().on_click(cx.listener(|this, _, _, cx| {
          this.review_status = ReviewStatus::All;
          this.review_rating = None;
          cx.notify();
        })))
        .into_any_element()
    } else {
      v_flex().gap_3().children(visible.iter().map(|review| self.review_card(review, cx)).collect::<Vec<_>>()).into_any_element()
    };
    let count = if filtered {
      trf(l, "ofReviews", &[&visible.len().to_string(), &reviews.len().to_string()])
    } else {
      trf(l, "reviews", &[&reviews.len().to_string()])
    };
    v_flex()
      .gap_5()
      .child(self.reviews_summary(reviews, cx))
      .child(
        h_flex()
          .w_full()
          .items_center()
          .justify_between()
          .child(h_flex().gap_1().children(ReviewStatus::ALL.map(|option| {
            Button::new(SharedString::from(format!("review-status-{}", option.label())))
              .label(tr(l, option.label()))
              .small()
              .outline()
              .selected(status == option)
              .on_click(cx.listener(move |this, _, _, cx| {
                this.review_status = option;
                cx.notify();
              }))
          })))
          .child(div().text_size(FONT_SIZE_SM).text_color(c.text_secondary).child(count)),
      )
      .child(list)
  }

  /// The average rating and how many reviews gave each number of stars; a row filters the list to that rating.
  fn reviews_summary(&self, reviews: &[CustomerReview], cx: &mut Context<Self>) -> Div {
    let l = self.settings.language;
    let c = colors(cx);
    let total = reviews.len().max(1);
    let average = reviews.iter().map(|review| review.rating).sum::<u64>() as f32 / total as f32;
    let unanswered = reviews.iter().filter(|review| review.response.is_none()).count();
    let selected = self.review_rating;
    let rows = (1..=5u64).rev().map(|n| {
      let count = reviews.iter().filter(|review| review.rating == n).count();
      let active = selected == Some(n);
      h_flex()
        .id(SharedString::from(format!("review-rating-{n}")))
        .w_full()
        .gap_3()
        .px_2()
        .py_0p5()
        .items_center()
        .rounded_md()
        .cursor_pointer()
        .text_size(FONT_SIZE_SM)
        .when(active, |row| row.bg(c.selected))
        .when(!active, |row| row.hover(move |row| row.bg(c.hover)))
        .child(div().w_4().text_color(c.text_secondary).child(format!("{n}★")))
        .child(
          div()
            .flex_1()
            .h_2()
            .rounded_full()
            .bg(c.border)
            .child(div().h_full().rounded_full().bg(c.warning).w(relative(count as f32 / total as f32))),
        )
        .child(div().w_8().text_color(c.text_secondary).child(count.to_string()))
        .on_click(cx.listener(move |this, _, _, cx| {
          this.review_rating = if this.review_rating == Some(n) { None } else { Some(n) };
          cx.notify();
        }))
    });
    h_flex()
      .w_full()
      .gap_8()
      .p_4()
      .rounded_lg()
      .border_1()
      .border_color(c.border)
      .items_center()
      .child(
        v_flex()
          .gap_1()
          .items_center()
          .w_40()
          .child(div().text_3xl().font_weight(FontWeight::SEMIBOLD).child(format!("{average:.1}")))
          .child(stars(average.round() as u64, &c))
          .child(div().text_size(FONT_SIZE_SM).text_color(c.text_secondary).child(trf(l, "reviews", &[&reviews.len().to_string()])))
          .child(div().text_size(FONT_SIZE_SM).text_color(if unanswered > 0 { c.warning } else { c.success }).child(trf(
            l,
            "unansweredCount",
            &[&unanswered.to_string()],
          ))),
      )
      .child(v_flex().flex_1().gap_0p5().children(rows))
  }

  fn review_card(&self, review: &CustomerReview, cx: &mut Context<Self>) -> impl IntoElement {
    let l = self.settings.language;
    let c = colors(cx);
    let target = review.clone();
    let reply = Button::new(SharedString::from(format!("reply-{}", review.id)))
      .label(tr(l, if review.response.is_some() { "editReply" } else { "reply" }))
      .small()
      .outline()
      .disabled(self.store_write_busy || self.saving)
      .on_click(cx.listener(move |this, _, window, cx| this.edit_customer_reply(&target, false, window, cx)));
    let mut body = v_flex().gap_2().child(
      h_flex()
        .gap_2()
        .items_center()
        .child(stars(review.rating, &c))
        .child(div().font_weight(FontWeight::SEMIBOLD).min_w_0().truncate().child(SharedString::from(review.title.clone()))),
    );
    body = body
      .child(div().text_size(FONT_SIZE_SM).text_color(c.text_secondary).child(format!(
        "{} · {} · {}",
        review.author,
        review.territory,
        review.created.split('T').next().unwrap_or_default()
      )))
      .child(div().pt_1().child(SharedString::from(review.body.clone())));
    let mut actions = h_flex().gap_2().pt_1().child(reply);
    if let Some(response) = &review.response {
      body = body.child(
        v_flex()
          .gap_1()
          .mt_1()
          .p_3()
          .rounded_md()
          .bg(c.surface)
          .child(
            h_flex().gap_2().text_size(FONT_SIZE_SM).text_color(c.text_secondary).child(tr(l, "yourReply")).child(state_badge(&response.state, &c)),
          )
          .child(SharedString::from(response.body.clone())),
      );
      let target = review.clone();
      actions = actions.child(
        Button::new(SharedString::from(format!("delete-reply-{}", review.id)))
          .label(tr(l, "deleteReply"))
          .small()
          .outline()
          .disabled(self.store_write_busy || self.saving)
          .on_click(cx.listener(move |this, _, window, cx| this.edit_customer_reply(&target, true, window, cx))),
      );
    }
    v_flex()
      .id(SharedString::from(format!("review-{}", review.id)))
      .w_full()
      .p_4()
      .gap_2()
      .rounded_lg()
      .border_1()
      .border_color(c.border)
      .child(body)
      .child(actions)
  }

  fn edit_customer_reply(&self, review: &CustomerReview, delete: bool, window: &mut Window, cx: &mut Context<Self>) {
    let l = self.settings.language;
    let field = if delete {
      None
    } else {
      let field = cx.new(|cx| TextareaState::new(window, cx).rows(5));
      if let Some(response) = &review.response {
        set_area(&field, &response.body, window, cx);
      }
      Some(field)
    };
    let change = if delete {
      let Some(response) = &review.response else { return };
      Change::DeleteReviewResponse(response.id.clone())
    } else {
      Change::PublishReviewResponse(review.id.clone())
    };
    let label = change.label();
    let title: SharedString = review.title.clone().into();
    let workspace = cx.entity();
    window.open_dialog(cx, move |dialog, _, cx| {
      let (workspace, change, field_for_save) = (workspace.clone(), change.clone(), field.clone());
      let footer = h_flex()
        .gap_2()
        .justify_end()
        .w_full()
        .child(Button::new("cancel-reply").label(tr(l, "cancel")).outline().on_click(|_, window, cx| window.close_dialog(cx)))
        .child(Button::new("confirm-reply").label(tr(l, label)).primary().on_click(move |_, window, cx| {
          let input = field_for_save.as_ref().map_or_else(String::new, |field| read_area(field, cx));
          window.close_dialog(cx);
          workspace.update(cx, |this, cx| this.apply_store_change(change.clone(), input, window, cx));
        }));
      let message = if delete { tr(l, "deleteYourPublicReply") } else { tr(l, "thisReplyWillAppear") };
      dialog
        .title(tr(l, label))
        .child(
          v_flex().gap_2().child(title.clone()).child(message).children(field.as_ref().map(|field| area_field(tr(l, "yourReply"), field, None, cx))),
        )
        .footer(footer)
    });
  }
}
