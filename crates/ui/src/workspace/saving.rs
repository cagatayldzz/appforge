//! Saving the pages to App Store Connect and the privacy answers to the settings file.

use super::*;

impl Workspace {
  pub(crate) fn run_save(
    &mut self,
    window: &mut Window,
    cx: &mut Context<Self>,
    work: impl FnOnce(&Client) -> Result<bool, appforge_appstore::Error> + Send + 'static,
  ) {
    let (Some(client), Some(id)) = (self.client.clone(), self.selected.clone()) else { return };
    if self.saving || self.store_write_busy {
      return;
    }
    self.saving = true;
    self.tasks.save = Some(cx.spawn_in(window, async move |this, cx| {
      let worker = client.clone();
      let result = cx.background_spawn(async move { work(&worker) }).await;
      this
        .update_in(cx, |this, window, cx| {
          this.saving = false;
          if !this.client.as_ref().is_some_and(|current| Arc::ptr_eq(current, &client)) {
            return;
          }
          if this.require_apple_sign_in(&result, window, cx) {
            return;
          }
          match result {
            Ok(true) => {
              Self::success(window, cx, tr(this.settings.language, "savedToAppStore"));
              if let Some(data) = this.app_data.get_mut(&id) {
                data.snapshot_task = None;
                data.product_assets.clear();
                if this.selected.as_ref() != Some(&id) {
                  data.snapshot = Load::Idle;
                }
              }
              if this.selected.as_ref() == Some(&id) {
                this.load_snapshot(window, cx);
              }
            }
            Ok(false) => Self::success(window, cx, tr(this.settings.language, "nothingChanged")),
            Err(error) => Self::error(window, cx, error.to_string()),
          }
          cx.notify();
        })
        .ok();
    }));
    cx.notify();
  }
}
