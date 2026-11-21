//! The History page: for each iOS version, what happened to it, newest first, as far as the App Store Connect API tells it.

use crate::{
  i18n::{tr, trf},
  review_submissions::submitted_at,
  theme::{colors, Colors, FONT_SIZE_SM},
  widgets::state_badge,
  workspace::{Load, Workspace},
};
use appforge_appstore::{Distribution, StoreVersion};
use gpui_kit::{
  component::{alert::Alert, h_flex, spinner::Spinner, v_flex},
  *,
};

/// One line of a version's history. `by` and `date` are empty when the API does not say.
struct Activity {
  state: String,
  by: String,
  date: String,
}

/// What happened to `version`, newest first: it was created, each submission for review was sent, and how each one ended.
/// Apple's own steps (in review, the verdict) carry no date in the API, so they show without one.
fn activities(distribution: &Distribution, version: &StoreVersion) -> Vec<Activity> {
  let mut list = vec![Activity { state: "PREPARE_FOR_SUBMISSION".into(), by: String::new(), date: version.created.clone() }];
  let mut submissions: Vec<_> =
    distribution.submissions.iter().filter(|item| !item.date.is_empty() && item.versions.contains(&version.number)).collect();
  submissions.sort_by(|a, b| a.date.cmp(&b.date));
  for submission in submissions {
    list.push(Activity { state: "WAITING_FOR_REVIEW".into(), by: submission.submitted_by.clone(), date: submission.date.clone() });
    let ended = match submission.state.as_str() {
      _ if submission.removed => Some("DEVELOPER_REJECTED"),
      "UNRESOLVED_ISSUES" => Some("REJECTED"),
      "COMPLETE" => Some("ACCEPTED"),
      "IN_REVIEW" => Some("IN_REVIEW"),
      _ => None,
    };
    if let Some(state) = ended {
      list.push(Activity { state: state.into(), by: "Apple".into(), date: String::new() });
    }
  }
  // Where the version stands now, when that is not already the last line.
  if list.len() > 1 && list.last().is_some_and(|last| last.state != version.state) {
    list.push(Activity { state: version.state.clone(), by: String::new(), date: String::new() });
  }
  list.reverse();
  list
}

impl Workspace {
  pub(crate) fn render_history(&self, cx: &mut Context<Self>) -> AnyElement {
    let l = self.settings.language;
    let c = colors(cx);
    let data = match self.active_data().map(|data| &data.distribution) {
      Some(Load::Ready(data)) => data,
      Some(Load::Failed(message)) => return Alert::error("history-error", message.clone()).into_any_element(),
      _ => return h_flex().gap_2().child(Spinner::new()).child(tr(l, "loadingFromAppStore")).into_any_element(),
    };
    let mut page = v_flex().gap_8();
    for version in &data.versions {
      let rows = activities(data, version).into_iter().map(|item| {
        let date = if item.date.is_empty() { "—".to_owned() } else { submitted_at(&item.date) };
        let by = if item.by.is_empty() { "—".to_owned() } else { item.by.clone() };
        row([state_badge(&item.state, &c).into_any_element(), by.into_any_element(), date.into_any_element()], false, &c)
      });
      let header =
        row(["activityColumn", "userColumn", "dateColumn"].map(|key| SharedString::from(tr(l, key).to_uppercase()).into_any_element()), true, &c);
      page = page.child(
        v_flex()
          .gap_3()
          .child(div().text_xl().font_weight(FontWeight::SEMIBOLD).child(trf(l, "version", &[&version.number])))
          .child(v_flex().w_full().border_t_1().border_color(c.border).child(header).children(rows)),
      );
    }
    if data.versions.is_empty() {
      page = page.child(div().text_color(c.text_secondary).child(tr(l, "noVersionsYet")));
    }
    page.into_any_element()
  }
}

fn row(cells: [AnyElement; 3], header: bool, c: &Colors) -> Div {
  let cell = |content: AnyElement| {
    let cell = div().flex_1().min_w_0().truncate().child(content);
    if header {
      cell.text_size(FONT_SIZE_SM).text_color(c.text_secondary)
    } else {
      cell
    }
  };
  let [first, second, third] = cells;
  h_flex().w_full().gap_3().px_3().py_3().items_center().border_b_1().border_color(c.border).child(cell(first)).child(cell(second)).child(cell(third))
}
