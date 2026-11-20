//! Background update checks, verified downloads and the settings controls.

use super::Workspace;
use crate::{
  i18n::{tr, trf},
  theme::{colors, FONT_SIZE_SM},
};
use appforge_updater::{Release, Staged};
use gpui_kit::{
  component::{
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    h_flex,
    separator::Separator,
    v_flex, Disableable as _, Icon, IconName, Sizable as _,
  },
  *,
};

#[derive(Default)]
pub(super) struct Updates {
  status: Status,
  release: Option<Release>,
  staged: Option<Staged>,
  task: Option<Task<()>>,
}

#[derive(Default)]
enum Status {
  #[default]
  Idle,
  Checking,
  Current,
  Available,
  Downloading,
  Staged,
  Restarting,
  Failed(SharedString),
}

impl Workspace {
  pub(super) fn check_updates(&mut self, manual: bool, window: &mut Window, cx: &mut Context<Self>) {
    if cfg!(debug_assertions) || self.updates.task.is_some() || self.updates.staged.is_some() {
      return;
    }
    self.updates.status = Status::Checking;
    self.updates.release = None;
    self.updates.task = Some(cx.spawn_in(window, async move |this, cx| {
      let result = cx
        .background_spawn(async {
          appforge_updater::current_app_bundle()?;
          appforge_updater::check(env!("CARGO_PKG_VERSION"))
        })
        .await;
      this
        .update_in(cx, |this, window, cx| {
          this.updates.task = None;
          let l = this.settings.language;
          this.updates.status = match result {
            Ok(Some(release)) => {
              Self::success(window, cx, trf(l, "updateAvailable", &[&release.version]));
              this.updates.release = Some(release);
              Status::Available
            }
            Ok(None) => Status::Current,
            Err(error) => {
              if manual {
                Self::error(window, cx, trf(l, "updateFailed", &[&error.to_string()]));
              }
              Status::Failed(error.to_string().into())
            }
          };
          cx.notify();
        })
        .ok();
    }));
    cx.notify();
  }

  fn download_update(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let Some(release) = self.updates.release.clone() else { return };
    if self.updates.task.is_some() || self.updates.staged.is_some() {
      return;
    }
    self.updates.status = Status::Downloading;
    self.updates.task = Some(cx.spawn_in(window, async move |this, cx| {
      let result = cx
        .background_spawn(async move {
          let app = appforge_updater::current_app_bundle()?;
          appforge_updater::prepare(&release, &app, |_| {})
        })
        .await;
      this
        .update_in(cx, |this, window, cx| {
          this.updates.task = None;
          this.updates.status = match result {
            Ok(staged) => {
              this.updates.staged = Some(staged);
              Status::Staged
            }
            Err(error) => {
              Self::error(window, cx, trf(this.settings.language, "updateFailed", &[&error.to_string()]));
              Status::Failed(error.to_string().into())
            }
          };
          cx.notify();
        })
        .ok();
    }));
    cx.notify();
  }

  fn restart_to_update(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    if self.updates.task.is_some() || self.saving || self.store_write_busy {
      return;
    }
    let Some(mut staged) = self.updates.staged.take() else { return };
    self.updates.status = Status::Restarting;
    self.updates.task = Some(cx.spawn_in(window, async move |this, cx| {
      let (staged, result) = cx
        .background_spawn(async move {
          let result = appforge_updater::current_app_bundle().and_then(|app| appforge_updater::install(&mut staged, &app));
          (staged, result)
        })
        .await;
      this
        .update_in(cx, |this, window, cx| match result {
          Ok(()) => cx.quit(),
          Err(error) => {
            this.updates.task = None;
            this.updates.staged = Some(staged);
            Self::error(window, cx, trf(this.settings.language, "updateFailed", &[&error.to_string()]));
            this.updates.status = Status::Failed(error.to_string().into());
            cx.notify();
          }
        })
        .ok();
    }));
    cx.notify();
  }

  /// The updates card of About: what the state is in a colored dot and a sentence, one main action for the next step,
  /// the notes of the release that is available, and the check on launch.
  pub(crate) fn render_updates(&self, cx: &mut Context<Self>) -> impl IntoElement {
    let l = self.settings.language;
    let c = colors(cx);
    let busy = self.updates.task.is_some();
    let staged = self.updates.staged.is_some();
    let (dot, status) = match &self.updates.status {
      Status::Idle if cfg!(debug_assertions) => (c.text_secondary, tr(l, "updatesReleaseOnly")),
      Status::Idle => (c.text_secondary, tr(l, "updatesIdle")),
      Status::Checking => (c.info, tr(l, "checkingForUpdates")),
      Status::Current => (c.success, tr(l, "upToDate")),
      Status::Available => (c.accent, trf(l, "updateAvailable", &[self.updates.release.as_ref().map_or("", |release| &release.version)])),
      Status::Downloading => (c.info, tr(l, "downloadingUpdate")),
      Status::Staged => (c.success, tr(l, "updateReady")),
      Status::Restarting => (c.info, tr(l, "restartingToUpdate")),
      Status::Failed(error) => (c.danger, trf(l, "updateFailed", &[error])),
    };
    // One main action: install when ready, download when found, otherwise check again.
    let action = if staged {
      Button::new("restart-app-update")
        .icon(Icon::new(IconName::RefreshCw))
        .label(tr(l, "restartToUpdate"))
        .primary()
        .disabled(busy || self.saving || self.store_write_busy)
        .on_click(cx.listener(|this, _, window, cx| this.restart_to_update(window, cx)))
    } else if self.updates.release.is_some() {
      Button::new("download-app-update")
        .icon(Icon::new(IconName::ArrowDown))
        .label(tr(l, "downloadUpdate"))
        .primary()
        .loading(matches!(self.updates.status, Status::Downloading))
        .disabled(busy)
        .on_click(cx.listener(|this, _, window, cx| this.download_update(window, cx)))
    } else {
      Button::new("check-app-updates")
        .icon(Icon::new(IconName::RefreshCw))
        .label(tr(l, "checkForUpdates"))
        .outline()
        .loading(matches!(self.updates.status, Status::Checking))
        .disabled(cfg!(debug_assertions) || busy)
        .on_click(cx.listener(|this, _, window, cx| this.check_updates(true, window, cx)))
    };
    let notes = self.updates.release.as_ref().map(|release| {
      let page_url = release.page_url.clone();
      let notes_text = release.plain_notes();
      let lines = notes_text.lines().filter(|line| !line.trim().is_empty()).map(|line| match line.strip_prefix("• ") {
        Some(item) => h_flex()
          .gap_2()
          .items_start()
          .text_size(FONT_SIZE_SM)
          .child(div().pt_1p5().child(div().size_1().rounded_full().bg(c.text_secondary)))
          .child(div().flex_1().min_w_0().child(item.to_string()))
          .into_any_element(),
        None => {
          div().pt_1().text_size(FONT_SIZE_SM).font_weight(FontWeight::MEDIUM).text_color(c.text_secondary).child(line.to_string()).into_any_element()
        }
      });
      v_flex()
        .w_full()
        .gap_2()
        .child(
          h_flex()
            .w_full()
            .items_center()
            .justify_between()
            .child(div().font_weight(FontWeight::MEDIUM).child(trf(l, "whatsNewIn", &[&release.version])))
            .child(
              Button::new("open-release-page")
                .icon(Icon::new(IconName::ExternalLink))
                .label(tr(l, "viewOnGithub"))
                .ghost()
                .small()
                .on_click(move |_, _, cx| cx.open_url(&page_url)),
            ),
        )
        .child(
          v_flex()
            .id("release-notes")
            .max_h(rems(14.))
            .overflow_y_scroll()
            .gap_1()
            .p_3()
            .rounded_md()
            .border_1()
            .border_color(c.border)
            .bg(c.surface_alt)
            .children(lines.collect::<Vec<_>>()),
        )
    });
    v_flex()
      .w_full()
      .gap_4()
      .child(
        h_flex()
          .w_full()
          .gap_3()
          .flex_wrap()
          .items_center()
          .justify_between()
          .child(
            h_flex()
              .flex_1()
              .flex_basis(rems(12.))
              .min_w_0()
              .gap_2()
              .items_center()
              .child(div().flex_none().size_2().rounded_full().bg(dot))
              .child(div().min_w_0().child(status)),
          )
          .child(action),
      )
      .children(notes)
      .child(Separator::horizontal())
      .child(
        Checkbox::new("check-updates-on-launch").label(tr(l, "checkUpdatesOnLaunch")).checked(!self.settings.skip_update_check_on_launch).on_click(
          cx.listener(|this, checked, window, cx| {
            this.settings.skip_update_check_on_launch = !checked;
            this.save_settings(window, cx);
            cx.notify();
          }),
        ),
      )
  }
}
