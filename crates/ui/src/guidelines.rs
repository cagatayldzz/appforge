use crate::{
  analysis::ago,
  forms::read_input,
  i18n::{tr, trf},
  widgets::{section, text_field},
  workspace::Workspace,
};
use appforge_compliance::{Guidelines, GUIDELINES_URL};
use gpui_kit::{
  component::{alert::Alert, button::Button, h_flex, v_flex, Sizable as _},
  *,
};
use std::{
  sync::Arc,
  time::{SystemTime, UNIX_EPOCH},
};

impl Workspace {
  pub(crate) fn refresh_guidelines(&mut self, manual: bool, window: &mut Window, cx: &mut Context<Self>) {
    if self.guidelines_busy {
      return;
    }
    let path = self.data_dir.clone();
    self.guidelines_busy = true;
    self.tasks.guidelines = Some(cx.spawn_in(window, async move |this, cx| {
      let cache_path = path.clone();
      let cached = Arc::new(cx.background_spawn(async move { Guidelines::load(&cache_path) }).await);
      this
        .update_in(cx, |this, _, cx| {
          this.guidelines = cached.clone();
          cx.notify();
        })
        .ok();
      let result = cx
        .background_spawn(async move {
          let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
          let refresh = manual || now.saturating_sub(cached.checked_at) >= 86_400;
          refresh.then(|| Guidelines::refresh(&path, &cached)).transpose()
        })
        .await;
      this
        .update_in(cx, |this, window, cx| {
          this.guidelines_busy = false;
          match result {
            Ok(updated) => {
              if let Some(updated) = updated {
                this.guidelines = Arc::new(updated);
              }
              this.guidelines_error = None;
              if manual {
                Self::success(window, cx, tr(this.settings.language, "Apple guidelines checked."));
              }
            }
            Err(error) => {
              this.guidelines_error = Some(error.clone().into());
              if manual {
                Self::error(window, cx, error);
              }
            }
          }
          cx.notify();
        })
        .ok();
    }));
    cx.notify();
  }

  pub(crate) fn render_guidelines(&self, cx: &mut Context<Self>) -> impl IntoElement {
    let l = self.settings.language;
    let changes = &self.guidelines.changes;
    let search = read_input(&self.guideline_search, cx).to_lowercase();
    let rules: Vec<_> = self
      .guidelines
      .rules
      .iter()
      .filter(|rule| {
        search.split_whitespace().all(|term| [&rule.number, &rule.title, &rule.topic].iter().any(|value| value.to_lowercase().contains(term)))
      })
      .collect();
    let mut content = v_flex()
      .gap_3()
      .child(
        h_flex()
          .gap_2()
          .flex_wrap()
          .child(
            Button::new("refresh-apple-guidelines")
              .label(tr(l, "Update rules"))
              .small()
              .outline()
              .loading(self.guidelines_busy)
              .on_click(cx.listener(|this, _, window, cx| this.refresh_guidelines(true, window, cx))),
          )
          .child(
            Button::new("open-apple-guidelines").label(tr(l, "View on Apple")).small().outline().on_click(|_, _, cx| cx.open_url(GUIDELINES_URL)),
          ),
      )
      .children(self.guidelines_error.as_ref().map(|error| Alert::error("apple-guidelines-error", error.clone())));
    if self.guidelines.checked_at > 0 {
      content = content.child(trf(l, "Last checked {}", &[&ago(l, self.guidelines.checked_at)]));
    }
    for (label, ids) in [("Added rules", &changes.added), ("Changed rules", &changes.modified), ("Removed rules", &changes.removed)] {
      if !ids.is_empty() {
        content = content.child(div().text_sm().child(format!("{} · {}", tr(l, label), ids.join(", "))));
      }
    }
    content = content.child(text_field(tr(l, "Search by rule number or topic"), &self.guideline_search, None, cx)).child(trf(
      l,
      "{} matching rules",
      &[&rules.len().to_string()],
    ));
    for rule in rules {
      let title = if rule.title.is_empty() { &rule.topic } else { &rule.title };
      let url = rule.url();
      content = content.child(
        h_flex().gap_2().justify_between().child(div().min_w_0().child(format!("{} · {title}", rule.number))).child(
          Button::new(SharedString::from(format!("apple-rule-{}", rule.number)))
            .label(tr(l, "View on Apple"))
            .small()
            .outline()
            .on_click(move |_, _, cx| cx.open_url(&url)),
        ),
      );
    }
    section(tr(l, "Apple App Review Guidelines"), content)
  }
}
