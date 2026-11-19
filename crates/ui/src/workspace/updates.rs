//! Background update checks, verified downloads and the settings controls.

use super::Workspace;
use crate::i18n::{tr, trf};
use appforge_updater::{Release, Staged};
use gpui_kit::{
  component::{button::Button, checkbox::Checkbox, h_flex, v_flex, Disableable as _, Sizable as _},
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

  pub(crate) fn render_updates(&self, cx: &mut Context<Self>) -> impl IntoElement {
    let l = self.settings.language;
    let busy = self.updates.task.is_some();
    let staged = self.updates.staged.is_some();
    let status = match &self.updates.status {
      Status::Idle if cfg!(debug_assertions) => tr(l, "updatesReleaseOnly"),
      Status::Idle => tr(l, "updatesIdle"),
      Status::Checking => tr(l, "checkingForUpdates"),
      Status::Current => tr(l, "upToDate"),
      Status::Available => trf(l, "updateAvailable", &[self.updates.release.as_ref().map_or("", |release| &release.version)]),
      Status::Downloading => tr(l, "downloadingUpdate"),
      Status::Staged => tr(l, "updateReady"),
      Status::Restarting => tr(l, "restartingToUpdate"),
      Status::Failed(error) => trf(l, "updateFailed", &[error]),
    };
    v_flex()
      .gap_2()
      .pb_4()
      .child(div().font_weight(FontWeight::SEMIBOLD).child(tr(l, "appUpdates")))
      .child(div().text_sm().child(status))
      .children(self.updates.release.as_ref().map(|release| div().text_sm().child(release.plain_notes())))
      .child(
        h_flex()
          .gap_2()
          .flex_wrap()
          .child(
            Button::new("check-app-updates")
              .small()
              .label(tr(l, "checkForUpdates"))
              .disabled(cfg!(debug_assertions) || busy || staged)
              .on_click(cx.listener(|this, _, window, cx| this.check_updates(true, window, cx))),
          )
          .children((self.updates.release.is_some() && !staged).then(|| {
            Button::new("download-app-update")
              .small()
              .label(tr(l, "downloadUpdate"))
              .disabled(busy)
              .on_click(cx.listener(|this, _, window, cx| this.download_update(window, cx)))
          }))
          .children(staged.then(|| {
            Button::new("restart-app-update")
              .small()
              .label(tr(l, "restartToUpdate"))
              .disabled(busy || self.saving || self.store_write_busy)
              .on_click(cx.listener(|this, _, window, cx| this.restart_to_update(window, cx)))
          })),
      )
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
