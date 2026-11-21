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
    button::Button,
    h_flex,
    scroll::ScrollableElement as _,
    sidebar::{SidebarItem as _, SidebarMenu, SidebarMenuItem},
    skeleton::Skeleton,
    tooltip::Tooltip,
    v_flex, Disableable as _, Icon, IconName, Sizable as _,
  },
  prelude::FluentBuilder as _,
  *,
};

/// Apple's dark "New" pill beside a page.
fn new_badge(c: &crate::theme::Colors, label: SharedString) -> impl Fn(&mut Window, &mut App) -> Div + 'static {
  let (background, text) = (c.control_border, c.text);
  move |_, _| div().px_2().rounded_full().bg(background).text_color(text).text_size(FONT_SIZE_SM).child(label.clone())
}

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
  pub(crate) fn render_sidebar(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
      (self.app_identity(&app, cx), self.page_rows(window, cx), Some(self.page_actions(cx)))
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
    let loading = self.active_data().is_some_and(|data| matches!(data.snapshot, Load::Idle | Load::Loading));
    let placeholder = || {
      h_flex()
        .w_full()
        .gap_3()
        .p_2()
        .items_center()
        .rounded_lg()
        .border_1()
        .border_color(c.border)
        .child(Skeleton::new().size_12().rounded_lg().flex_none())
        .child(v_flex().flex_1().min_w_0().gap_2().child(Skeleton::new().h_4().w_20()).child(Skeleton::new().h_3().w_32().max_w_full()))
    };
    let status = h_flex()
      .id("score-badge")
      .w_full()
      .gap_3()
      .p_2()
      .items_center()
      .rounded_lg()
      .border_1()
      .border_color(c.border)
      .cursor_pointer()
      .hover(move |tile| tile.bg(c.hover))
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
      .children(loading.then(placeholder))
      .children((!loading && (self.report.is_some() || version.is_some())).then_some(status))
      .into_any_element()
  }

  /// The pages of the open app, grouped the way App Store Connect groups them. A page with sub-pages opens and closes like a tree.
  fn page_rows(&self, window: &mut Window, cx: &mut Context<Self>) -> Vec<AnyElement> {
    let l = self.settings.language;
    let c = colors(cx);
    let group_title =
      |text: SharedString| div().px_2().pt_3().pb_1().text_size(FONT_SIZE).font_weight(FontWeight::SEMIBOLD).child(text).into_any_element();
    let group_label =
      |text: SharedString| div().px_2().pt_3().pb_1().text_size(FONT_SIZE_SM).text_color(c.text_secondary).child(text).into_any_element();
    let divider = || div().mx_2().mt_2().h_px().bg(c.border).into_any_element();
    let version_chip = self.version_chip(cx);
    // Without a version there is no version page to open; the Product Page row stays as the way in.
    let version = if version_chip.is_some() { Vec::new() } else { vec![self.page_item(DetailTab::ProductPage, cx)] };
    // A red dot beside App Review while Apple's last answer for this version is a rejection, as in App Store Connect.
    let rejected = matches!(
      self.active_data().map(|data| &data.distribution),
      Some(crate::workspace::Load::Ready(data)) if data.submissions.iter().any(|item| item.includes_version && item.state == "UNRESOLVED_ISSUES")
    );
    let general = vec![
      self.page_item(DetailTab::Information, cx),
      self.page_item(DetailTab::Compliance, cx).suffix(new_badge(&c, tr(self.settings.language, "newBadge"))),
      // App Review is the list of submissions, as in App Store Connect; the review contact and notes are on the version page.
      self.page_link(tr(l, "appReview"), self.tab == DetailTab::Submissions, None, DetailTab::Submissions, cx).when(rejected, |item| {
        let danger = c.danger;
        item.suffix(move |_, _| div().size_2().flex_none().rounded_full().bg(danger))
      }),
      self.page_link(tr(l, "history"), self.tab == DetailTab::History, None, DetailTab::History, cx),
    ];
    let trust = [DetailTab::Privacy, DetailTab::CustomerReviews].map(|tab| self.page_item(tab, cx)).to_vec();
    let monetization = vec![self.page_item(DetailTab::Pricing, cx)];
    let add_platform = self.selected.clone().map(|id| {
      div()
        .id("add-platform")
        .mx_2()
        .mt_2()
        .px_2()
        .text_size(FONT_SIZE_SM)
        .cursor_pointer()
        .text_color(c.accent)
        .child(tr(l, "addPlatform"))
        .on_click(move |_, _, cx| cx.open_url(&format!("https://appstoreconnect.apple.com/apps/{id}/distribution")))
        .into_any_element()
    });
    let create_version = self.create_version_link(cx);
    let mut menu = |id: &'static str, items: Vec<SidebarMenuItem>| {
      div().pl_2().child(SidebarMenu::new().gap_0p5().children(items).render(id, window, cx)).into_any_element()
    };
    vec![group_title(tr(l, "sidebarIosApp")), version_chip.unwrap_or_else(|| menu("pages-version", version))]
      .into_iter()
      .chain(add_platform)
      .chain(create_version)
      .chain([
        divider(),
        group_title(tr(l, "general")),
        menu("pages-general", general),
        divider(),
        group_title(tr(l, "sidebarAppStore")),
        group_label(tr(l, "sidebarTrustAndSafety")),
        menu("pages-trust", trust),
        group_label(tr(l, "sidebarMonetization")),
        menu("pages-monetization", monetization),
      ])
      .collect()
  }

  /// The version under "iOS App", the way App Store Connect shows it: a grey chip with where the version stands at Apple. It opens the version page.
  fn version_chip(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
    let c = colors(cx);
    let version = self.ready_snapshot().and_then(|snapshot| snapshot.version.as_ref())?;
    let selected = self.tab == DetailTab::ProductPage;
    let label = format!("{} {}", version.version_string, crate::widgets::state_label(&version.state));
    Some(
      h_flex()
        .id("page-version")
        .mx_2()
        .mt_1()
        .px_2()
        .py_1p5()
        .gap_2()
        .items_center()
        .rounded_md()
        .cursor_pointer()
        .text_size(FONT_SIZE_SM)
        .bg(if selected { c.selected } else { c.hover })
        .hover(move |chip| chip.bg(c.selected))
        .child(div().size_2().flex_none().rounded_full().bg(crate::widgets::state_color(&version.state, &c)))
        .child(div().min_w_0().truncate().child(label))
        .on_click(cx.listener(|this, _, window, cx| {
          this.tab = DetailTab::ProductPage;
          this.back_tab = None;
          this.ensure_app_page(window, cx);
          cx.notify();
        }))
        .into_any_element(),
    )
  }

  /// One page of the open app: its name and, when something waits on it, a status dot. Analysis has no item; the score tile opens it.
  fn page_item(&self, tab: DetailTab, cx: &mut Context<Self>) -> SidebarMenuItem {
    let l = self.settings.language;
    let status = self.page_needs_attention(tab, cx);
    self.page_link(tr(l, tab.label()), self.tab == tab, status, tab, cx)
  }

  /// A row that opens `tab`.
  fn page_link(&self, label: SharedString, active: bool, attention: Option<bool>, tab: DetailTab, cx: &mut Context<Self>) -> SidebarMenuItem {
    let l = self.settings.language;
    let c = colors(cx);
    let workspace = cx.entity();
    let status = attention.map(|attention| if attention { (c.warning, tr(l, "pageStatusNeedsAttention")) } else { (c.success, tr(l, "ready")) });
    SidebarMenuItem::new(label)
      .active(active)
      .when_some(status, |item, (color, label)| {
        item.suffix(move |_, _| {
          let label = label.clone();
          div()
            .id("page-status")
            .size_2()
            .flex_none()
            .rounded_full()
            .bg(color)
            .tooltip(move |window, cx| Tooltip::new(label.clone()).build(window, cx))
        })
      })
      .on_click(move |_, window, cx| {
        workspace.update(cx, |this, cx| {
          this.tab = tab;
          this.back_tab = None;
          this.ensure_app_page(window, cx);
          cx.notify();
        })
      })
  }

  /// Actions for the open app, pinned below the sidebar pages.
  fn page_actions(&self, cx: &mut Context<Self>) -> AnyElement {
    let l = self.settings.language;
    let media_locale = self.product_localization();
    let busy = self.active_data().is_some_and(|data| match self.effective_tab() {
      DetailTab::CustomerReviews => data.reviews_task.is_some(),
      DetailTab::Pricing => data.pricing_task.is_some(),
      DetailTab::ProductPage => {
        data.snapshot_task.is_some()
          || data.distribution_task.is_some()
          || media_locale.and_then(|locale| data.product_assets.get(locale)).is_some_and(|entry| entry.task.is_some())
      }
      DetailTab::Distribution | DetailTab::Submissions | DetailTab::History => data.snapshot_task.is_some() || data.distribution_task.is_some(),
      _ => data.snapshot_task.is_some(),
    });
    let reload = Button::new("reload-snapshot")
      .icon(Icon::new(IconName::RefreshCw))
      .label(tr(l, "reload"))
      .loading(busy)
      .disabled(busy || self.saving || self.store_write_busy || false)
      .outline()
      .small()
      .on_click(cx.listener(|this, _, window, cx| match this.effective_tab() {
        DetailTab::CustomerReviews => this.refresh_customer_reviews(true, window, cx),
        DetailTab::Pricing => this.refresh_pricing(true, window, cx),
        DetailTab::ProductPage if this.ready_snapshot().is_some() => {
          this.refresh_snapshot(Some(DetailTab::ProductPage), window, cx);
          this.load_product_assets(true, window, cx);
          this.refresh_distribution(true, window, cx);
        }
        DetailTab::Distribution | DetailTab::Submissions | DetailTab::History if this.ready_snapshot().is_some() => {
          this.refresh_distribution(true, window, cx)
        }
        tab => this.refresh_snapshot(Some(tab), window, cx),
      }));
    let save = self.save_control(cx);
    let analysis = Button::new("sidebar-project-analysis")
      .icon(Icon::new(IconName::Bot))
      .small()
      .outline()
      .tooltip(tr(l, "aiAnalysis"))
      .loading(matches!(self.ai, AiState::Running(_)) || self.tasks.ai_dialog.is_some())
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
