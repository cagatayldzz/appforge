//! Saving the pages to App Store Connect and the privacy answers to the settings file.

use super::*;

impl Workspace {
  pub(super) fn run_save(
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

  pub(super) fn edited_info(&self, privacy_url: Option<String>, cx: &App) -> Option<(AppInfoLocalization, AppInfoLocalization)> {
    let original = self.ready_snapshot()?.info_localizations.iter().find(|item| item.locale == self.locale)?.clone();
    let mut edited = original.clone();
    match privacy_url {
      Some(url) => edited.privacy_policy_url = url,
      None => {
        edited.name = read_input(&self.forms.name, cx);
        edited.subtitle = read_input(&self.forms.subtitle, cx);
      }
    }
    Some((edited, original))
  }

  pub fn save_information(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let info = self.edited_info(None, cx);
    let listing =
      self.ready_snapshot().and_then(|snapshot| snapshot.version_localizations.iter().find(|item| item.locale == self.locale)).cloned().map(
        |original| {
          let edited = VersionLocalization {
            description: read_area(&self.forms.description, cx),
            keywords: read_input(&self.forms.keywords, cx),
            promotional_text: read_area(&self.forms.promotional, cx),
            support_url: read_input(&self.forms.support_url, cx),
            marketing_url: read_input(&self.forms.marketing_url, cx),
            whats_new: read_area(&self.forms.whats_new, cx),
            ..original.clone()
          };
          (edited, original)
        },
      );
    self.run_save(window, cx, move |client| {
      let mut sent = false;
      if let Some((edited, original)) = &info {
        sent |= client.update_info_localization(edited, original)?;
      }
      if let Some((edited, original)) = &listing {
        sent |= client.update_version_localization(edited, original)?;
      }
      Ok(sent)
    });
  }

  pub fn save_review(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let Some(snapshot) = self.ready_snapshot() else { return };
    let Some(version) = snapshot.version.clone() else { return };
    let detail = ReviewDetail {
      id: snapshot.review_detail.as_ref().and_then(|detail| detail.id.clone()),
      contact_first_name: read_input(&self.forms.first_name, cx),
      contact_last_name: read_input(&self.forms.last_name, cx),
      contact_phone: read_input(&self.forms.phone, cx),
      contact_email: read_input(&self.forms.email, cx),
      demo_account_required: self.forms.demo_required,
      demo_account_name: read_input(&self.forms.demo_name, cx),
      demo_account_password: read_input(&self.forms.demo_password, cx),
      notes: read_area(&self.forms.notes, cx),
    };
    self.run_save(window, cx, move |client| client.save_review_detail(&version.id, &detail).map(|_| true));
  }

  /// Saves the local privacy answers and, when it changed, the privacy policy URL at Apple.
  pub fn save_privacy(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    if let Some(id) = self.selected.clone() {
      let mut declaration = self.privacy_declaration();
      declaration.data_types = read_input(&self.forms.data_types, cx);
      declaration.third_party_sdks = read_input(&self.forms.sdks, cx);
      self.settings.privacy.insert(id, declaration);
      self.save_settings(window, cx);
      self.recompute_report();
    }
    let info = self.edited_info(Some(read_input(&self.forms.privacy_url, cx)), cx);
    self.run_save(window, cx, move |client| match &info {
      Some((edited, original)) => client.update_info_localization(edited, original),
      None => Ok(false),
    });
    cx.notify();
  }

  /// Changes one of the privacy answers and saves it at once; the answers are local, so this is cheap.
  pub fn update_declaration(&mut self, window: &mut Window, cx: &mut Context<Self>, change: impl FnOnce(&mut PrivacyDeclaration)) {
    let Some(id) = self.selected.clone() else { return };
    let mut declaration = self.privacy_declaration();
    change(&mut declaration);
    self.settings.privacy.insert(id, declaration);
    self.save_settings(window, cx);
    self.recompute_report();
    cx.notify();
  }
}
