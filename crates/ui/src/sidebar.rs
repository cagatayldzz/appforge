//! The left side of the signed-in window: the activity bar (icon buttons), the sidebar (the apps, or the pages of Settings)
//! and the status bar at the bottom of the main area.

use crate::{
  i18n::{tr, trf},
  theme::{colors, FONT_SIZE, FONT_SIZE_SM},
  widgets::{app_avatar, list_row, rail_button, state_badge, ACTIVITY_BAR_WIDTH, SIDEBAR_WIDTH},
  workspace::{AiState, DetailTab, Load, SettingsPage, Workspace},
};
use appforge_compliance::Verdict;
use gpui_kit::{
  component::{
    button::Button, h_flex, scroll::ScrollableElement as _, tag::Tag, tooltip::Tooltip, v_flex, Disableable as _, Icon, IconName, Sizable as _,
  },
  prelude::FluentBuilder as _,
  *,
};

impl Workspace {
  /// The strip on the far left: one avatar per app, and Settings at the bottom.
  pub(crate) fn render_rail(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
    let c = colors(cx);
    let l = self.settings.language;
    let workspace = cx.entity();
    let apps = self.app_list();
    let all_apps = {
      let workspace = workspace.clone();
      rail_button("rail-all-apps", IconName::LayoutDashboard, tr(l, "apps"), self.show_all_apps && !self.show_settings, cx, move |_, _, cx| {
        workspace.update(cx, |this, cx| {
          this.show_settings = false;
          this.show_all_apps = true;
          cx.notify();
        })
      })
    };
    let items: Vec<AnyElement> = apps
      .iter()
      .map(|app| {
        let id = app.id.clone();
        let selected = !self.show_settings && !self.show_all_apps && self.selected.as_ref() == Some(&id);
        let name: SharedString = app.name.clone().into();
        div()
          .id(SharedString::from(format!("rail-app-{}", app.id)))
          .group("rail-app")
          .flex_none()
          .rounded_md()
          .cursor_pointer()
          .child(
            app_avatar(&app.id, &app.name, selected, true, self.icons.get(&app.id), cx)
              .size_10()
              .when(!selected, |avatar| avatar.group_hover("rail-app", move |style| style.border_color(c.control_border))),
          )
          .tooltip(move |window, cx| Tooltip::new(name.clone()).build(window, cx))
          .on_click(cx.listener(move |this, _, window, cx| {
            this.show_settings = false;
            this.select_app(id.clone(), window, cx)
          }))
          .into_any_element()
      })
      .collect();
    let settings = rail_button("rail-settings", IconName::Settings, tr(l, "settings"), self.show_settings, cx, move |_, _, cx| {
      workspace.update(cx, |this, cx| {
        this.show_settings = true;
        cx.notify();
      })
    });
    v_flex()
      .w(px(ACTIVITY_BAR_WIDTH))
      .h_full()
      .flex_none()
      .items_center()
      .py_2()
      .gap_2()
      .bg(c.rail)
      .border_r_1()
      .border_color(c.border)
      .child(all_apps)
      .child(v_flex().id("rail-apps").flex_1().min_h_0().items_center().gap_2().overflow_y_scrollbar().children(items))
      .child(settings)
  }

  /// The column next to the activity bar: the apps, or the pages of Settings while Settings is open.
  pub(crate) fn render_sidebar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
    let c = colors(cx);
    let l = self.settings.language;
    let title = |text: SharedString, action: Option<AnyElement>| {
      h_flex()
        .h_10()
        .px_4()
        .flex_none()
        .items_center()
        .justify_between()
        .child(div().text_size(FONT_SIZE).font_weight(FontWeight::SEMIBOLD).child(text))
        .children(action)
        .into_any_element()
    };
    // The top of the column says what the main area shows: the pages of Settings, the app that is open, or all apps.
    let (header, rows, actions) = if self.show_settings {
      (title(tr(l, "settings"), None), self.settings_rows(cx), None)
    } else if let Some(app) = self.selected_app().filter(|_| !self.show_all_apps) {
      (self.app_identity(&app, cx), self.page_rows(cx), Some(self.page_actions(cx)))
    } else {
      (title(tr(l, "apps"), Some(self.reload_button(cx).into_any_element())), Vec::new(), None)
    };
    // Reload and Save sit at the bottom of the column, under a line.
    let footer = actions.map(|actions| v_flex().flex_none().border_t_1().border_color(c.border).child(actions));
    v_flex()
      .w(px(SIDEBAR_WIDTH))
      .h_full()
      .flex_none()
      .bg(c.surface)
      .border_r_1()
      .border_color(c.border)
      .child(header)
      .child(v_flex().id("sidebar-list").flex_1().min_h_0().px_2().pt_2().gap_0p5().overflow_y_scrollbar().children(rows))
      .children(footer)
  }

  pub(crate) fn reload_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
    let l = self.settings.language;
    Button::new("reload-apps")
      .icon(Icon::new(IconName::RefreshCw))
      .outline()
      .small()
      .loading(matches!(self.apps, Load::Loading))
      .tooltip(tr(l, "reloadApps"))
      .on_click(cx.listener(|this, _, window, cx| this.load_apps(window, cx)))
  }

  /// The app that is open: its avatar, name, bundle id, version and where the version stands at Apple.
  fn app_identity(&self, app: &appforge_appstore::App, cx: &mut Context<Self>) -> AnyElement {
    let c = colors(cx);
    let l = self.settings.language;
    let version = self.ready_snapshot().and_then(|snapshot| snapshot.version.as_ref());
    // The review score, in the color of the verdict.
    let score = self.report.as_ref().map(|report| {
      let color = match report.verdict() {
        Verdict::NotReady => c.danger,
        Verdict::NeedsAttention => c.warning,
        Verdict::LooksReady => c.success,
      };
      v_flex()
        .flex_none()
        .items_center()
        .justify_center()
        .size_12()
        .rounded_lg()
        .bg(color.opacity(0.14))
        .child(div().text_color(color).text_size(rems(1.25)).font_weight(FontWeight::BOLD).child(report.score().to_string()))
        .child(div().text_size(FONT_SIZE_SM).text_color(c.text_secondary).child(tr(l, "score")))
    });
    // One tile: the score, the version and where it stands at Apple. It opens the Analysis page.
    let tooltip = trf(l, "openPage", &[&tr(l, DetailTab::Analysis.label())]);
    let open = self.tab == DetailTab::Analysis;
    let status = h_flex()
      .id("score-badge")
      .w_full()
      .gap_3()
      .p_2()
      .items_center()
      .rounded_lg()
      .border_1()
      .border_color(if open { c.control_border } else { c.border })
      .cursor_pointer()
      .when(open, |tile| tile.bg(c.selected))
      .when(!open, |tile| tile.hover(move |tile| tile.bg(c.hover)))
      .children(score)
      .child(v_flex().flex_1().min_w_0().gap_1().text_size(FONT_SIZE_SM).children(version.map(|version| {
        v_flex()
          .gap_1()
          .child(div().text_size(FONT_SIZE).font_weight(FontWeight::SEMIBOLD).text_color(c.text).child(trf(l, "version", &[&version.version_string])))
          .child(div().text_color(c.text_secondary).child(state_badge(&version.state, &c)))
      })))
      .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
      .on_click(cx.listener(|this, _, window, cx| this.jump_to(DetailTab::Analysis, window, cx)));
    v_flex()
      .w_full()
      .flex_none()
      .gap_3()
      .p_4()
      .border_b_1()
      .border_color(c.border)
      .child(
        h_flex().gap_3().items_center().child(app_avatar(&app.id, &app.name, true, false, self.icons.get(&app.id), cx).size_10()).child(
          v_flex()
            .flex_1()
            .min_w_0()
            .child(div().truncate().text_size(FONT_SIZE).font_weight(FontWeight::SEMIBOLD).child(app.name.clone()))
            .child(div().truncate().text_size(FONT_SIZE_SM).text_color(c.text_secondary).child(app.bundle_id.clone())),
        ),
      )
      .child(status)
      .into_any_element()
  }

  /// The pages of the open app, one under the other.
  fn page_rows(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
    let l = self.settings.language;
    let c = colors(cx);
    DetailTab::ALL
      .iter()
      .map(|tab| {
        let tab = *tab;
        list_row(SharedString::from(format!("page-{}", tab.label())), self.tab == tab, cx)
          .py_2()
          .justify_between()
          .child(div().min_w_0().truncate().child(tr(l, tab.label())))
          .child(
            h_flex()
              .gap_1()
              .flex_none()
              .children(self.page_completion(tab, cx).map(|(done, total)| {
                let color = if done == total { c.success } else { c.warning };
                Tag::custom(color.opacity(0.16), c.text, color).small().child(format!("{done}/{total}"))
              }))
              .children({
                let count = self.ai_tasks_for(tab).count();
                (count > 0).then(|| Tag::custom(c.accent.opacity(0.16), c.text, c.accent).small().child(trf(l, "aiTaskCount", &[&count.to_string()])))
              }),
          )
          .on_click(cx.listener(move |this, _, window, cx| {
            this.tab = tab;
            this.back_tab = None;
            this.ensure_app_page(window, cx);
            cx.notify();
          }))
          .into_any_element()
      })
      .collect()
  }

  /// Actions for the open app, pinned below the sidebar pages.
  fn page_actions(&self, cx: &mut Context<Self>) -> AnyElement {
    let l = self.settings.language;
    let busy = self.active_data().is_some_and(|data| match self.tab {
      DetailTab::CustomerReviews => data.reviews_task.is_some(),
      DetailTab::Readiness | DetailTab::Distribution => data.snapshot_task.is_some() || data.distribution_task.is_some(),
      _ => data.snapshot_task.is_some(),
    });
    let reload = Button::new("reload-snapshot").icon(Icon::new(IconName::RefreshCw)).label(tr(l, "reload")).loading(busy).outline().small().on_click(
      cx.listener(|this, _, window, cx| match this.tab {
        DetailTab::CustomerReviews => this.load_customer_reviews(window, cx),
        DetailTab::ProductPage => this.load_product_assets(true, window, cx),
        DetailTab::Distribution if this.ready_snapshot().is_some() => this.load_distribution(window, cx),
        DetailTab::Readiness => {
          this.load_snapshot(window, cx);
          this.load_distribution(window, cx);
        }
        _ => this.load_snapshot(window, cx),
      }),
    );
    let save = self.save_control(cx);
    let analysis = Button::new("sidebar-project-analysis")
      .icon(Icon::new(IconName::Bot))
      .small()
      .outline()
      .tooltip(tr(l, "aiAnalysis"))
      .loading(matches!(self.ai, AiState::Running))
      .disabled(self.ready_snapshot().is_none())
      .on_click(cx.listener(|this, _, window, cx| this.confirm_ai(true, window, cx)));
    v_flex()
      .gap_2()
      .px_4()
      .py_3()
      .child(h_flex().gap_2().child(reload).child(analysis).children(save.map(|button| button.flex_1())))
      .into_any_element()
  }

  fn settings_rows(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
    let l = self.settings.language;
    SettingsPage::ALL
      .iter()
      .map(|page| {
        let page = *page;
        list_row(SharedString::from(format!("settings-page-{}", page.label())), self.settings_page == page, cx)
          .py_2()
          .child(tr(l, page.label()))
          .on_click(cx.listener(move |this, _, _, cx| {
            this.settings_page = page;
            cx.notify();
          }))
          .into_any_element()
      })
      .collect()
  }
}
