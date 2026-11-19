//! Startup (the Keychain) and signing in and out of App Store Connect.

use super::*;

impl Workspace {
  pub(super) fn restore(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let secrets = self.secrets.clone();
    let signed_in_before = self.settings.apple.is_some();
    self.tasks.restore = Some(cx.spawn_in(window, async move |this, cx| {
      let loaded = cx
        .background_spawn(async move {
          let key = if signed_in_before { secrets.get(SecretKey::ApplePrivateKey) } else { Ok(None) };
          let keys: Vec<(Provider, bool)> =
            Provider::ALL.iter().map(|provider| (*provider, matches!(secrets.get(SecretKey::ApiKey(*provider)), Ok(Some(_))))).collect();
          (key, keys)
        })
        .await;
      this.update_in(cx, |this, window, cx| this.finish_restore(loaded.0.map_err(|error| error.to_string()), loaded.1, window, cx)).ok();
    }));
    self.refresh_playbook_if_stale(window, cx);
    self.refresh_guidelines(false, window, cx);
  }

  pub(super) fn finish_restore(
    &mut self,
    key: Result<Option<String>, String>,
    keys: Vec<(Provider, bool)>,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    self.starting = false;
    self.has_key = keys.into_iter().collect();
    match (key, self.settings.apple.clone()) {
      (Ok(Some(pem)), Some(account)) => {
        self.client = Some(Arc::new(Client::new(Credentials { issuer_id: account.issuer_id, key_id: account.key_id, private_key: pem })));
        self.load_apps(window, cx);
      }
      (Err(error), _) => {
        Self::error(window, cx, trf(self.settings.language, "Could not read the App Store Connect key from the Keychain: {}", &[&error]))
      }
      _ => {}
    }
    cx.notify();
  }

  pub fn choose_key_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let receiver = cx.prompt_for_paths(PathPromptOptions {
      files: true,
      directories: false,
      multiple: false,
      prompt: Some(tr(self.settings.language, "Choose key")),
    });
    self.tasks.key_file = Some(cx.spawn_in(window, async move |this, cx| {
      if let Ok(Ok(Some(paths))) = receiver.await {
        if let Some(path) = paths.into_iter().next() {
          this.update_in(cx, |this, window, cx| this.key_file_chosen(path, window, cx)).ok();
        }
      }
    }));
  }

  pub(super) fn key_file_chosen(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
    let file_name = path.file_name().map(|name| name.to_string_lossy().to_string()).unwrap_or_default();
    let pem = match std::fs::read_to_string(&path) {
      Ok(pem) => pem,
      Err(error) => {
        self.login.error = Some(trf(self.settings.language, "Could not read {}: {}", &[&file_name, &error.to_string()]));
        cx.notify();
        return;
      }
    };
    if let Err(error) = appforge_appstore::validate_private_key(&pem) {
      self.login.error = Some(error.to_string().into());
      Self::error(window, cx, error.to_string());
      cx.notify();
      return;
    }
    // Apple names the file AuthKey_<KEY ID>.p8, so the Key ID can be filled in.
    if read_input(&self.login.key_id, cx).is_empty() {
      if let Some(key_id) = file_name.strip_prefix("AuthKey_").and_then(|rest| rest.strip_suffix(".p8")) {
        set_input(&self.login.key_id, key_id, window, cx);
      }
    }
    self.login.key = Some(LoginKey { file_name, pem });
    self.login.error = None;
    cx.notify();
  }

  pub fn sign_in(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    if self.login.busy || self.settings_locked {
      return;
    }
    let issuer_id = read_input(&self.login.issuer, cx);
    let key_id = read_input(&self.login.key_id, cx);
    let l = self.settings.language;
    let missing = if issuer_id.is_empty() {
      Some(tr(l, "Enter your Issuer ID."))
    } else if key_id.is_empty() {
      Some(tr(l, "Enter your Key ID."))
    } else if self.login.key.is_none() {
      Some(tr(l, "Choose your .p8 private key file."))
    } else {
      None
    };
    if let Some(message) = missing {
      self.login.error = Some(message);
      cx.notify();
      return;
    }
    let pem = self.login.key.as_ref().map(|key| key.pem.clone()).unwrap_or_default();
    let credentials = Credentials { issuer_id: issuer_id.clone(), key_id: key_id.clone(), private_key: pem.clone() };
    self.login.busy = true;
    self.login.error = None;
    self.tasks.sign_in = Some(cx.spawn_in(window, async move |this, cx| {
      let result = cx
        .background_spawn(async move {
          let client = Client::new(credentials);
          client.verify().map(|()| client)
        })
        .await;
      this
        .update_in(cx, |this, window, cx| {
          this.login.busy = false;
          match result {
            Ok(client) => this.signed_in(AppleAccount { issuer_id, key_id }, pem, client, window, cx),
            Err(error) => {
              this.login.error = Some(error.to_string().into());
              Self::error(window, cx, error.to_string());
            }
          }
          cx.notify();
        })
        .ok();
    }));
    cx.notify();
  }

  pub(super) fn signed_in(&mut self, account: AppleAccount, pem: String, client: Client, window: &mut Window, cx: &mut Context<Self>) {
    self.settings.apple = Some(account);
    self.settings.onboarded = true;
    self.save_settings(window, cx);
    let l = self.settings.language;
    let secrets = self.secrets.clone();
    self.tasks.secrets = Some(cx.spawn_in(window, async move |this, cx| {
      let result = cx.background_spawn(async move { secrets.set(SecretKey::ApplePrivateKey, &pem) }).await;
      if let Err(error) = result {
        this
          .update_in(cx, |_, window, cx| {
            Self::error(window, cx, trf(l, "Signed in, but the key could not be saved in the Keychain: {}", &[&error.to_string()]))
          })
          .ok();
      }
    }));
    self.client = Some(Arc::new(client));
    self.login.key = None;
    self.load_apps(window, cx);
  }

  pub fn sign_out(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let l = self.settings.language;
    let secrets = self.secrets.clone();
    self.tasks.secrets = Some(cx.spawn_in(window, async move |this, cx| {
      if let Err(error) = cx.background_spawn(async move { secrets.delete(SecretKey::ApplePrivateKey) }).await {
        this
          .update_in(cx, |_, window, cx| Self::error(window, cx, trf(l, "The key could not be removed from the Keychain: {}", &[&error.to_string()])))
          .ok();
      }
    }));
    self.clear_apple_session(window, cx);
  }

  pub(crate) fn require_apple_sign_in<T>(
    &mut self,
    result: &Result<T, appforge_appstore::Error>,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) -> bool {
    if !matches!(result, Err(appforge_appstore::Error::Api { status: 401, .. })) {
      return false;
    }
    if self.client.is_some() {
      let account = self.settings.apple.clone();
      window.close_dialog(cx);
      self.settings.onboarded = true;
      self.clear_apple_session(window, cx);
      if let Some(account) = account {
        set_input(&self.login.issuer, &account.issuer_id, window, cx);
        set_input(&self.login.key_id, &account.key_id, window, cx);
      }
      self.login.step = 1;
      self.login.key = None;
      self.login.error =
        Some(tr(self.settings.language, "App Store Connect rejected your session. Confirm your IDs and choose a valid .p8 key to sign in again."));
      cx.notify();
    }
    true
  }

  pub(super) fn clear_apple_session(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    self.settings.apple = None;
    self.save_settings(window, cx);
    self.client = None;
    self.tasks.ai = None;
    self.tasks.save = None;
    self.tasks.store_write = None;
    self.saving = false;
    self.store_write_busy = false;
    self.apps = Load::Idle;
    self.icons.clear();
    self.tasks.icons = None;
    self.versions.clear();
    self.tasks.versions = None;
    self.selected = None;
    self.app_data.clear();
    self.tasks.draft = None;
    self.drafting = None;
    self.report = None;
    self.ai = AiState::Idle;
    self.show_settings = false;
    self.tasks.apps = None;
    cx.notify();
  }
}
