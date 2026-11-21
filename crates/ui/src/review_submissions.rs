//! The App Review page: the submissions sent to App Review, as App Store Connect lists them under App Review.

use crate::{
  distribution::Change,
  i18n::{tr, trf},
  theme::{colors, Colors},
  widgets::state_color,
  workspace::{Load, Workspace},
};
use appforge_appstore::{App as StoreApp, ReviewSubmission};
use chrono::{DateTime, Local};
use gpui_kit::{
  component::{alert::Alert, h_flex, separator::Separator, spinner::Spinner, v_flex},
  *,
};

/// "2026-11-08T10:53:35.538Z" in the local time zone as "Nov 8 at 1:53 PM"; the text itself when it is not a date.
pub(crate) fn submitted_at(value: &str) -> String {
  match DateTime::parse_from_rfc3339(value) {
    Ok(date) => date.with_timezone(&Local).format("%b, %-d at %-I:%M %p").to_string(),
    Err(_) => value.to_owned(),
  }
}

/// The status as App Store Connect words it: a red dot for unresolved issues, a grey one for a submission that was removed.
fn status(submission: &ReviewSubmission, c: &Colors) -> Div {
  let (color, label) = if submission.removed {
    (c.text_secondary, "Removed".to_owned())
  } else {
    let label = submission.state.split('_').map(|word| word[..1].to_uppercase() + &word[1..].to_lowercase()).collect::<Vec<_>>().join(" ");
    (if submission.state == "UNRESOLVED_ISSUES" { c.danger } else { state_color(&submission.state, c) }, label)
  };
  h_flex().gap_1p5().items_center().child(div().size_2().rounded_full().bg(color)).child(div().child(label))
}

impl Workspace {
  pub(crate) fn render_review_submissions(&self, app: &StoreApp, cx: &mut Context<Self>) -> AnyElement {
    let l = self.settings.language;
    let c = colors(cx);
    let data = match self.active_data().map(|data| &data.distribution) {
      Some(Load::Ready(data)) => data,
      Some(Load::Failed(message)) => return Alert::error("submissions-error", message.clone()).into_any_element(),
      _ => return h_flex().gap_2().child(Spinner::new()).child(tr(l, "loadingFromAppStore")).into_any_element(),
    };
    // A draft that nothing was sent with is not a submission yet; App Store Connect leaves it out as well.
    let mut submissions: Vec<&ReviewSubmission> = data.submissions.iter().filter(|item| !item.date.is_empty()).collect();
    submissions.sort_by(|a, b| b.date.cmp(&a.date));
    let cell = |text: SharedString| div().flex_1().min_w_0().truncate().child(text);
    let header = h_flex()
      .w_full()
      .gap_3()
      .px_3()
      .py_2()
      .text_size(crate::theme::FONT_SIZE_SM)
      .text_color(c.text_secondary)
      .border_b_1()
      .border_color(c.border)
      .children(["dateSubmitted", "versionsColumn", "submittedBy", "itemsColumn", "statusColumn"].map(|key| cell(tr(l, key).to_uppercase().into())));
    let mut table = v_flex().w_full().child(header);
    for submission in submissions {
      let url = format!("https://appstoreconnect.apple.com/apps/{}/distribution/reviewsubmissions/details/{}", app.id, submission.id);
      let versions = submission.versions.iter().map(|version| format!("iOS {version}")).collect::<Vec<_>>().join(", ");
      let items = if submission.item_count == 1 { tr(l, "oneItem") } else { trf(l, "manyItems", &[&submission.item_count.to_string()]) };
      table = table.child(
        h_flex()
          .id(SharedString::from(format!("submission-{}", submission.id)))
          .w_full()
          .gap_3()
          .px_3()
          .py_3()
          .items_center()
          .border_b_1()
          .border_color(c.border)
          .child(
            div()
              .id(SharedString::from(format!("open-submission-{}", submission.id)))
              .flex_1()
              .min_w_0()
              .truncate()
              .cursor_pointer()
              .text_color(c.accent)
              .child(submitted_at(&submission.date))
              .on_click(move |_, _, cx| cx.open_url(&url)),
          )
          .child(cell(versions.into()))
          .child(cell(submission.submitted_by.clone().into()))
          .child(cell(items))
          .child(div().flex_1().min_w_0().child(status(submission, &c)))
          .children((!submission.removed && matches!(submission.state.as_str(), "WAITING_FOR_REVIEW" | "IN_REVIEW" | "UNRESOLVED_ISSUES")).then(
            || {
              self.distribution_button(
                SharedString::from(format!("withdraw-{}", submission.id)),
                Change::Cancel(submission.id.clone(), submission.item_count),
                cx,
              )
            },
          )),
      );
    }
    let empty = data.submissions.iter().all(|item| item.date.is_empty());
    let body = if empty {
      div().text_color(c.text_secondary).child(tr(l, "nothingWasSubmitted")).into_any_element()
    } else {
      table.border_t_1().border_color(c.border).into_any_element()
    };
    // Spaced as in App Store Connect: the intro, a line, then the Submissions heading with its text and the table.
    v_flex()
      .gap_6()
      .child(div().text_color(c.text_secondary).child(tr(l, "appReviewIntro")))
      .child(Separator::horizontal())
      .child(
        v_flex()
          .gap_3()
          .child(div().text_xl().font_weight(FontWeight::SEMIBOLD).child(tr(l, "submissions")))
          .child(div().text_sm().text_color(c.text_secondary).child(tr(l, "submissionsHelp")))
          .child(div().pt_2().child(body)),
      )
      .into_any_element()
  }
}
