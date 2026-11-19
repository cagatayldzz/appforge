//! Review cards and the dialogs for publishing or deleting developer replies.

use crate::{
  distribution::Change,
  forms::{read_area, set_area},
  i18n::tr,
  theme::{colors, Colors, FONT_SIZE_SM},
  widgets::{area_field, state_badge},
  workspace::Workspace,
};
use appforge_appstore::CustomerReview;
use gpui_kit::{
  component::{
    button::{Button, ButtonVariants as _},
    h_flex,
    input::TextareaState,
    v_flex, Disableable as _, Sizable as _, WindowExt as _,
  },
  *,
};

/// The stars of a rating: filled ones in the warning color, the rest faint.
fn stars(rating: u64, c: &Colors) -> Div {
  h_flex().gap_0p5().text_color(c.warning).children((1..=5u64).map(|n| div().text_color(if n <= rating { c.warning } else { c.border }).child("★")))
}

impl Workspace {
  pub(super) fn review_card(&self, review: &CustomerReview, cx: &mut Context<Self>) -> impl IntoElement {
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
      v_flex()
        .gap_2()
        .items_start()
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
    let mut actions = h_flex().gap_2().pt_1().flex_wrap().child(reply);
    if let Some(app) = &self.selected {
      let url = format!("https://appstoreconnect.apple.com/apps/{app}/distribution");
      actions = actions.child(Button::new("report-review").label(tr(l, "reportReview")).small().ghost().on_click(move |_, _, cx| cx.open_url(&url)));
    }
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
