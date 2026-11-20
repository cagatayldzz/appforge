//! One finding of the Analysis page as a card, and the findings grouped by severity.

use crate::{
  analysis::{page_of, severity_color, SEVERITIES},
  finding_fix::fixable,
  i18n::{tr, translated_text, trf},
  theme::{colors, FONT_SIZE_SM},
  widgets::severity_badge,
  workspace::Workspace,
};
use appforge_compliance::{Finding, Severity};
use gpui_kit::{
  component::{
    button::{Button, ButtonVariants as _},
    h_flex, v_flex, Disableable as _, Icon, IconName, Sizable as _,
  },
  *,
};

impl Workspace {
  /// Critical findings and warnings start open, tips closed; a click flips that for one finding.
  fn finding_open(&self, key: &str, severity: Severity) -> bool {
    (severity != Severity::Info) != self.toggled_findings.contains(key)
  }

  /// A card with a bar in the color of its severity. The header opens and closes it; open, it shows what is wrong,
  /// how to fix it, and the actions: fix it with the AI, open the page, read Apple's rule.
  fn finding_row(&self, scope: &'static str, finding: &Finding, cx: &mut Context<Self>) -> impl IntoElement {
    let l = self.settings.language;
    let c = colors(cx);
    let key = format!("{scope}-{}-{}", finding.id, finding.title);
    let open = self.finding_open(&key, finding.severity);
    let tab = page_of(finding.category);
    let toggle_key = key.clone();
    let mut meta = h_flex()
      .gap_2()
      .items_center()
      .flex_wrap()
      .child(severity_badge(finding.severity, l, cx))
      .child(div().text_size(FONT_SIZE_SM).text_color(c.text_secondary).child(tr(l, finding.category.label())));
    if !finding.guideline.is_empty() {
      meta =
        meta.child(div().text_size(FONT_SIZE_SM).text_color(c.text_secondary).child(format!("· {}", trf(l, "guideline", &[&finding.guideline]))));
    }
    let header = h_flex()
      .id(SharedString::from(format!("finding-{key}")))
      .w_full()
      .gap_2()
      .items_start()
      .cursor_pointer()
      .child(div().pt_0p5().child(Icon::new(if open { IconName::ChevronDown } else { IconName::ChevronRight }).small().text_color(c.text_secondary)))
      .child(v_flex().flex_1().min_w_0().gap_1().child(div().font_weight(FontWeight::SEMIBOLD).child(translated_text(l, &finding.title))).child(meta))
      .on_click(cx.listener(move |this, _, _, cx| {
        if !this.toggled_findings.remove(&toggle_key) {
          this.toggled_findings.insert(toggle_key.clone());
        }
        cx.notify();
      }));
    let mut body = v_flex().w_full().p_3().gap_3().child(header);
    if open {
      let source = self.guidelines.rule(&finding.guideline).map(|rule| {
        let url = rule.url();
        Button::new(SharedString::from(format!("guideline-{key}")))
          .label(tr(l, "viewAppleRule"))
          .small()
          .ghost()
          .on_click(move |_, _, cx| cx.open_url(&url))
      });
      let fix_key = format!("{}-{}", finding.id, finding.title);
      let busy = self.fixing.is_some();
      let fix = fixable(finding).then(|| {
        let finding = finding.clone();
        Button::new(SharedString::from(format!("ai-fix-{key}")))
          .icon(Icon::new(IconName::Bot))
          .label(tr(l, "fixWithAi"))
          .small()
          .primary()
          .loading(self.fixing.as_ref() == Some(&fix_key))
          .disabled(busy)
          .on_click(cx.listener(move |this, _, window, cx| this.confirm_finding_fix(finding.clone(), window, cx)))
      });
      body = body
        .child(div().pl_6().text_color(c.text_secondary).child(translated_text(l, &finding.detail)))
        .child(
          v_flex()
            .ml_6()
            .p_3()
            .gap_1()
            .rounded_md()
            .bg(c.surface)
            .child(div().text_size(FONT_SIZE_SM).font_weight(FontWeight::SEMIBOLD).text_color(c.text_secondary).child(tr(l, "howToFix")))
            .child(div().child(translated_text(l, &finding.fix))),
        )
        .child(
          h_flex()
            .pl_6()
            .gap_2()
            .flex_wrap()
            .items_center()
            .children(fix)
            .child(
              Button::new(SharedString::from(format!("go-{key}")))
                .label(trf(l, "openPage", &[&tr(l, finding.category.label())]))
                .small()
                .outline()
                .on_click(cx.listener(move |this, _, window, cx| {
                  this.jump_to(tab, window, cx);
                })),
            )
            .children(source),
        );
    }
    h_flex()
      .w_full()
      .rounded_lg()
      .border_1()
      .border_color(c.border)
      .overflow_hidden()
      .child(div().w_1().flex_none().self_stretch().bg(severity_color(finding.severity, &c)))
      .child(div().flex_1().min_w_0().child(body))
  }

  /// Findings under a heading per severity, the worst first.
  pub(crate) fn finding_groups(&self, scope: &'static str, findings: &[Finding], cx: &mut Context<Self>) -> Div {
    let l = self.settings.language;
    let c = colors(cx);
    let mut groups = v_flex().w_full().gap_5();
    for severity in SEVERITIES {
      let items: Vec<&Finding> = findings.iter().filter(|finding| finding.severity == severity).collect();
      if items.is_empty() {
        continue;
      }
      let heading = h_flex()
        .gap_2()
        .items_center()
        .child(div().size_2().rounded_full().bg(severity_color(severity, &c)))
        .child(div().font_weight(FontWeight::SEMIBOLD).child(format!("{} · {}", tr(l, severity.label()), items.len())));
      groups = groups.child(v_flex().gap_2().child(heading).children(items.into_iter().map(|finding| self.finding_row(scope, finding, cx))));
    }
    groups
  }
}
