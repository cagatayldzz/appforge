use crate::{
  distribution::Change,
  forms::{read_area, set_area},
  i18n::tr,
  theme::colors,
  widgets::{area_field, section, state_badge},
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
    v_flex, Disableable as _, Sizable as _, WindowExt as _,
  },
  *,
};

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
      Some(Load::Ready(reviews)) if reviews.is_empty() => div().child(tr(l, "No customer reviews.")).into_any_element(),
      Some(Load::Ready(reviews)) => {
        v_flex().gap_6().children(reviews.iter().map(|review| self.review_card(review, cx)).collect::<Vec<_>>()).into_any_element()
      }
      Some(Load::Failed(message)) => v_flex()
        .gap_3()
        .child(Alert::error("customer-reviews-error", message.clone()))
        .child(
          h_flex().child(
            Button::new("retry-customer-reviews")
              .label(tr(l, "Try again"))
              .outline()
              .on_click(cx.listener(|this, _, window, cx| this.load_customer_reviews(window, cx))),
          ),
        )
        .into_any_element(),
      _ => h_flex().gap_2().child(Spinner::new()).child(tr(l, "Loading from App Store Connect…")).into_any_element(),
    }
  }

  fn review_card(&self, review: &CustomerReview, cx: &mut Context<Self>) -> Div {
    let l = self.settings.language;
    let palette = colors(cx);
    let target = review.clone();
    let reply = Button::new(SharedString::from(format!("reply-{}", review.id)))
      .label(tr(l, if review.response.is_some() { "Edit reply" } else { "Reply" }))
      .small()
      .outline()
      .disabled(self.store_write_busy || self.saving)
      .on_click(cx.listener(move |this, _, window, cx| this.edit_customer_reply(&target, false, window, cx)));
    let mut body = v_flex()
      .gap_3()
      .child(div().text_sm().text_color(palette.text_secondary).child(format!(
        "{}/5 · {} · {} · {}",
        review.rating,
        review.author,
        review.territory,
        review.created.split('T').next().unwrap_or_default()
      )))
      .child(SharedString::from(review.body.clone()));
    let mut actions = h_flex().gap_2().child(reply);
    if let Some(response) = &review.response {
      body = body.child(
        v_flex()
          .gap_1()
          .p_3()
          .rounded_md()
          .bg(palette.surface)
          .child(h_flex().gap_2().child(tr(l, "Your reply")).child(state_badge(&response.state, &palette)))
          .child(SharedString::from(response.body.clone())),
      );
      let target = review.clone();
      actions = actions.child(
        Button::new(SharedString::from(format!("delete-reply-{}", review.id)))
          .label(tr(l, "Delete reply"))
          .small()
          .outline()
          .disabled(self.store_write_busy || self.saving)
          .on_click(cx.listener(move |this, _, window, cx| this.edit_customer_reply(&target, true, window, cx))),
      );
    }
    section(SharedString::from(review.title.clone()), body.child(actions))
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
        .child(Button::new("cancel-reply").label(tr(l, "Cancel")).outline().on_click(|_, window, cx| window.close_dialog(cx)))
        .child(Button::new("confirm-reply").label(tr(l, label)).primary().on_click(move |_, window, cx| {
          let input = field_for_save.as_ref().map_or_else(String::new, |field| read_area(field, cx));
          window.close_dialog(cx);
          workspace.update(cx, |this, cx| this.apply_store_change(change.clone(), input, window, cx));
        }));
      let message =
        if delete { tr(l, "Delete your public reply to this review?") } else { tr(l, "This reply will appear publicly on the App Store.") };
      dialog
        .title(tr(l, label))
        .child(
          v_flex().gap_2().child(title.clone()).child(message).children(field.as_ref().map(|field| area_field(tr(l, "Your reply"), field, None, cx))),
        )
        .footer(footer)
    });
  }
}
