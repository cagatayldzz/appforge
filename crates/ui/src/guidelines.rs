//! The Apple guidelines browser in Settings: search, sections, update.

use crate::{
  analysis::ago,
  forms::read_input,
  i18n::{tr, trf},
  widgets::text_field,
  workspace::Workspace,
};
use appforge_compliance::{Guideline, Guidelines, GUIDELINES_URL};
use gpui_kit::{
  component::{
    alert::Alert,
    button::{Button, ButtonVariants as _},
    collapsible::Collapsible,
    group_box::{GroupBox, GroupBoxVariants as _},
    h_flex, v_flex, ActiveTheme as _, Disableable as _, Icon, IconName, Sizable as _,
  },
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
                Self::success(window, cx, tr(this.settings.language, "appleGuidelinesChecked"));
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
    let muted = cx.theme().muted_foreground;
    let changes = &self.guidelines.changes;
    let search = read_input(&self.guideline_search, cx).to_lowercase();
    let mut groups = v_flex().gap_2();
    let mut matches = 0;
    for (prefix, name) in [('1', "safety"), ('2', "performance"), ('3', "business"), ('4', "design"), ('5', "legal")] {
      let localized = tr(l, name);
      let rules: Vec<_> = self
        .guidelines
        .rules
        .iter()
        .filter(|rule| {
          rule.number.starts_with(prefix)
            && search.split_whitespace().all(|term| {
              [rule.number.as_str(), &rule.title, &rule.topic, name, localized.as_ref()].iter().any(|value| value.to_lowercase().contains(term))
            })
        })
        .collect();
      if rules.is_empty() {
        continue;
      }
      matches += rules.len();
      let searching = !search.is_empty();
      let open = searching || self.guideline_section == Some(prefix);
      let title = format!("{localized} · {}", rules.len());
      let mut group = Collapsible::new().open(open).child(
        Button::new(SharedString::from(format!("guideline-group-{prefix}")))
          .w_full()
          .ghost()
          .label(title)
          .icon(Icon::new(if open { IconName::ChevronDown } else { IconName::ChevronRight }))
          .disabled(searching)
          .on_click(cx.listener(move |this, _, _, cx| {
            this.guideline_section = (this.guideline_section != Some(prefix)).then_some(prefix);
            cx.notify();
          })),
      );
      if open {
        group = group.content(v_flex().children(rules.into_iter().map(|rule| self.guideline_row(rule, searching, cx))));
      }
      groups = groups.child(GroupBox::new().outline().child(group));
    }
    let count = if self.guidelines.rules.is_empty() {
      if self.guidelines_busy {
        tr(l, "loadingAppleRules")
      } else {
        tr(l, "noRulesDownloadedYet")
      }
    } else if search.is_empty() {
      trf(l, "rulesCount", &[&matches.to_string()])
    } else if matches == 0 {
      tr(l, "noMatchingRules")
    } else {
      trf(l, "matchingRules", &[&matches.to_string()])
    };
    let status = if self.guidelines.checked_at > 0 { Some(trf(l, "lastChecked", &[&ago(l, self.guidelines.checked_at)])) } else { None };
    let toolbar = h_flex()
      .gap_2()
      .flex_wrap()
      .child(
        Button::new("refresh-apple-guidelines")
          .icon(Icon::new(IconName::RefreshCw))
          .label(tr(l, "updateRules"))
          .small()
          .outline()
          .loading(self.guidelines_busy)
          .on_click(cx.listener(|this, _, window, cx| this.refresh_guidelines(true, window, cx))),
      )
      .child(
        Button::new("open-apple-guidelines")
          .icon(Icon::new(IconName::ExternalLink))
          .label(tr(l, "viewOnApple"))
          .small()
          .ghost()
          .on_click(|_, _, cx| cx.open_url(GUIDELINES_URL)),
      );
    let mut content = v_flex()
      .gap_3()
      .child(
        h_flex()
          .gap_3()
          .justify_between()
          .flex_wrap()
          .child(
            v_flex()
              .gap_1()
              .child(div().font_weight(FontWeight::SEMIBOLD).child(tr(l, "appleAppReviewGuidelines")))
              .children(status.map(|status| div().text_sm().text_color(muted).child(status))),
          )
          .child(toolbar),
      )
      .children(self.guidelines_error.as_ref().map(|error| Alert::error("apple-guidelines-error", error.clone())));
    for (label, ids) in [("addedRules", &changes.added), ("changedRules", &changes.modified), ("removedRules", &changes.removed)] {
      if !ids.is_empty() {
        content = content.child(div().text_sm().child(format!("{} · {}", tr(l, label), ids.join(", "))));
      }
    }
    content
      .child(text_field(tr(l, "searchByRuleNumber"), &self.guideline_search, None, cx))
      .child(div().text_sm().text_color(muted).child(count))
      .child(groups)
  }

  fn guideline_row(&self, rule: &Guideline, searching: bool, cx: &App) -> Div {
    let l = self.settings.language;
    let theme = cx.theme();
    let title = if rule.title.is_empty() && searching { &rule.topic } else { &rule.title };
    let url = rule.url();
    h_flex()
      .justify_between()
      .gap_2()
      .py_1()
      .px_2()
      .border_t_1()
      .border_color(theme.border)
      .child(
        h_flex()
          .min_w_0()
          .gap_2()
          .child(div().flex_none().text_sm().font_weight(FontWeight::SEMIBOLD).child(rule.number.clone()))
          .children((!title.is_empty()).then(|| div().text_sm().child(title.clone()))),
      )
      .child(
        Button::new(SharedString::from(format!("apple-rule-{}", rule.number)))
          .icon(Icon::new(IconName::ExternalLink))
          .label(tr(l, "openRule"))
          .ghost()
          .xsmall()
          .on_click(move |_, _, cx| cx.open_url(&url)),
      )
  }
}
