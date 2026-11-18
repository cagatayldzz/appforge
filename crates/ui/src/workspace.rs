//! The window's state: the session, the app list, the selected app and every background task.

use crate::forms::{read_area, read_input, set_input, Forms};
use appforge_ai::Provider;
use appforge_appstore::{App as StoreApp, AppInfoLocalization, AppSnapshot, Client, Credentials, ReviewDetail, VersionLocalization};
use appforge_compliance::{analyze_with_ai, check, AiReview, Playbook, Report};
use appforge_storage::{AppleAccount, Keychain, PrivacyDeclaration, SecretKey, SecretStore, Settings};
use gpui_kit::{
  component::{input::InputState, notification::Notification, WindowExt as _},
  *,
};
use std::{
  collections::HashMap,
  path::PathBuf,
  sync::Arc,
  time::{SystemTime, UNIX_EPOCH},
};

/// The cached playbook is refreshed in the background when it is older than this.
const PLAYBOOK_MAX_AGE_SECONDS: u64 = 7 * 24 * 60 * 60;

pub enum Load<T> {
  Idle,
  Loading,
  Ready(T),
  Failed(SharedString),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DetailTab {
  Information,
  Review,
  Privacy,
  Analysis,
}

impl DetailTab {
  pub const ALL: [DetailTab; 4] = [Self::Information, Self::Review, Self::Privacy, Self::Analysis];

  pub fn label(self) -> &'static str {
    match self {
      Self::Information => "App Information",
      Self::Review => "App Review",
      Self::Privacy => "App Privacy",
      Self::Analysis => "Analysis",
    }
  }
}

pub enum AiState {
  Idle,
  Running,
  Done(AiReview),
  Failed(SharedString),
}

pub struct LoginKey {
  pub file_name: String,
  pub pem: String,
}

pub struct LoginForm {
  pub issuer: Entity<InputState>,
  pub key_id: Entity<InputState>,
  pub key: Option<LoginKey>,
  pub busy: bool,
  pub error: Option<SharedString>,
}

pub struct SettingsForm {
  pub openai_key: Entity<InputState>,
  pub anthropic_key: Entity<InputState>,
  pub openai_model: Entity<InputState>,
  pub anthropic_model: Entity<InputState>,
}

/// Tasks live on the workspace; replacing one cancels the previous one.
#[derive(Default)]
struct Tasks {
  restore: Option<Task<()>>,
  sign_in: Option<Task<()>>,
  key_file: Option<Task<()>>,
  secrets: Option<Task<()>>,
  apps: Option<Task<()>>,
  snapshot: Option<Task<()>>,
  save: Option<Task<()>>,
  ai: Option<Task<()>>,
  playbook: Option<Task<()>>,
}

pub struct Workspace {
  pub data_dir: PathBuf,
  pub settings: Settings,
  /// The settings file exists but could not be read: nothing is saved over it.
  pub settings_locked: bool,
  secrets: Arc<dyn SecretStore>,
  pub client: Option<Arc<Client>>,
  /// Reading the Keychain at startup.
  pub starting: bool,
  pub apps: Load<Vec<StoreApp>>,
  pub selected: Option<String>,
  pub snapshot: Load<AppSnapshot>,
  pub tab: DetailTab,
  pub locale: String,
  pub report: Option<Report>,
  pub ai: AiState,
  pub playbook: Playbook,
  pub playbook_busy: bool,
  pub playbook_error: Option<SharedString>,
  pub has_key: HashMap<Provider, bool>,
  pub show_settings: bool,
  pub saving: bool,
  pub login: LoginForm,
  pub forms: Forms,
  pub settings_form: SettingsForm,
  tasks: Tasks,
}

fn now() -> u64 {
  SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

/// Every locale App Store Connect has for the app, the primary one first.
pub fn locales(snapshot: &AppSnapshot) -> Vec<String> {
  let mut locales: Vec<String> = snapshot
    .info_localizations
    .iter()
    .map(|item| item.locale.clone())
    .chain(snapshot.version_localizations.iter().map(|item| item.locale.clone()))
    .collect();
  locales.sort();
  locales.dedup();
  if let Some(position) = locales.iter().position(|locale| *locale == snapshot.app.primary_locale) {
    let primary = locales.remove(position);
    locales.insert(0, primary);
  }
  locales
}

impl Workspace {
  pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
    let data_dir = appforge_storage::data_dir();
    let (settings, settings_locked) = match Settings::load(&data_dir.join("settings.json")) {
      Ok(settings) => (settings, false),
      Err(_) => (Settings::default(), true),
    };
    let playbook = Playbook::load(&data_dir);
    let login = LoginForm {
      issuer: cx.new(|cx| InputState::new(window, cx).placeholder("00000000-0000-0000-0000-000000000000")),
      key_id: cx.new(|cx| InputState::new(window, cx).placeholder("ABC123DEFG")),
      key: None,
      busy: false,
      error: None,
    };
    let model_input = |cx: &mut Context<Self>, window: &mut Window, value: &str| {
      let value = value.to_owned();
      cx.new(|cx| {
        let mut state = InputState::new(window, cx);
        state.set_value(value, window, cx);
        state
      })
    };
    let settings_form = SettingsForm {
      openai_key: cx.new(|cx| InputState::new(window, cx).placeholder("sk-…").masked(true)),
      anthropic_key: cx.new(|cx| InputState::new(window, cx).placeholder("sk-ant-…").masked(true)),
      openai_model: model_input(cx, window, &settings.ai.openai_model),
      anthropic_model: model_input(cx, window, &settings.ai.anthropic_model),
    };
    let forms = Forms::new(window, cx);
    let mut this = Self {
      data_dir,
      settings,
      settings_locked,
      secrets: Arc::new(Keychain),
      client: None,
      starting: true,
      apps: Load::Idle,
      selected: None,
      snapshot: Load::Idle,
      tab: DetailTab::Information,
      locale: String::new(),
      report: None,
      ai: AiState::Idle,
      playbook,
      playbook_busy: false,
      playbook_error: None,
      has_key: HashMap::new(),
      show_settings: false,
      saving: false,
      login,
      forms,
      settings_form,
      tasks: Tasks::default(),
    };
    if this.settings_locked {
      this.login.error = Some(
        "The settings file could not be read, so nothing will be saved. Fix or remove settings.json in the data folder and restart AppForge.".into(),
      );
    }
    this.restore(window, cx);
    this
  }

  fn error(window: &mut Window, cx: &mut App, message: impl Into<SharedString>) {
    window.push_notification(Notification::error(message), cx);
  }

  fn success(window: &mut Window, cx: &mut App, message: impl Into<SharedString>) {
    window.push_notification(Notification::success(message), cx);
  }

  pub fn save_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    if self.settings_locked {
      Self::error(window, cx, "The settings file could not be read, so this change was not saved.");
      return;
    }
    if let Err(error) = self.settings.save(&self.data_dir.join("settings.json")) {
      Self::error(window, cx, format!("Could not save settings: {error}"));
    }
  }

  // Startup and sign-in ---------------------------------------------------------------------------------------

  fn restore(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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
  }

  fn finish_restore(&mut self, key: Result<Option<String>, String>, keys: Vec<(Provider, bool)>, window: &mut Window, cx: &mut Context<Self>) {
    self.starting = false;
    self.has_key = keys.into_iter().collect();
    match (key, self.settings.apple.clone()) {
      (Ok(Some(pem)), Some(account)) => {
        self.client = Some(Arc::new(Client::new(Credentials { issuer_id: account.issuer_id, key_id: account.key_id, private_key: pem })));
        self.load_apps(window, cx);
      }
      (Err(error), _) => Self::error(window, cx, format!("Could not read the App Store Connect key from the Keychain: {error}")),
      _ => {}
    }
    cx.notify();
  }

  pub fn choose_key_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let receiver = cx.prompt_for_paths(PathPromptOptions { files: true, directories: false, multiple: false, prompt: Some("Choose key".into()) });
    self.tasks.key_file = Some(cx.spawn_in(window, async move |this, cx| {
      if let Ok(Ok(Some(paths))) = receiver.await {
        if let Some(path) = paths.into_iter().next() {
          this.update_in(cx, |this, window, cx| this.key_file_chosen(path, window, cx)).ok();
        }
      }
    }));
  }

  fn key_file_chosen(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
    let file_name = path.file_name().map(|name| name.to_string_lossy().to_string()).unwrap_or_default();
    let pem = match std::fs::read_to_string(&path) {
      Ok(pem) => pem,
      Err(error) => {
        self.login.error = Some(format!("Could not read {file_name}: {error}").into());
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
    let missing = if issuer_id.is_empty() {
      Some("Enter your Issuer ID.")
    } else if key_id.is_empty() {
      Some("Enter your Key ID.")
    } else if self.login.key.is_none() {
      Some("Choose your .p8 private key file.")
    } else {
      None
    };
    if let Some(message) = missing {
      self.login.error = Some(message.into());
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

  fn signed_in(&mut self, account: AppleAccount, pem: String, client: Client, window: &mut Window, cx: &mut Context<Self>) {
    self.settings.apple = Some(account);
    self.save_settings(window, cx);
    let secrets = self.secrets.clone();
    self.tasks.secrets = Some(cx.spawn_in(window, async move |this, cx| {
      let result = cx.background_spawn(async move { secrets.set(SecretKey::ApplePrivateKey, &pem) }).await;
      if let Err(error) = result {
        this
          .update_in(cx, |_, window, cx| Self::error(window, cx, format!("Signed in, but the key could not be saved in the Keychain: {error}")))
          .ok();
      }
    }));
    self.client = Some(Arc::new(client));
    self.login.key = None;
    self.load_apps(window, cx);
  }

  pub fn sign_out(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let secrets = self.secrets.clone();
    self.tasks.secrets = Some(cx.spawn_in(window, async move |this, cx| {
      if let Err(error) = cx.background_spawn(async move { secrets.delete(SecretKey::ApplePrivateKey) }).await {
        this.update_in(cx, |_, window, cx| Self::error(window, cx, format!("The key could not be removed from the Keychain: {error}"))).ok();
      }
    }));
    self.settings.apple = None;
    self.save_settings(window, cx);
    self.client = None;
    self.apps = Load::Idle;
    self.selected = None;
    self.snapshot = Load::Idle;
    self.report = None;
    self.ai = AiState::Idle;
    self.show_settings = false;
    self.tasks.apps = None;
    self.tasks.snapshot = None;
    cx.notify();
  }

  // Apps ------------------------------------------------------------------------------------------------------

  pub fn load_apps(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let Some(client) = self.client.clone() else { return };
    self.apps = Load::Loading;
    self.tasks.apps = Some(cx.spawn_in(window, async move |this, cx| {
      let result = cx.background_spawn(async move { client.apps() }).await;
      this
        .update_in(cx, |this, window, cx| {
          match result {
            Ok(apps) => {
              let first = apps.first().map(|app| app.id.clone());
              this.apps = Load::Ready(apps);
              if this.selected.is_none() {
                if let Some(id) = first {
                  this.select_app(id, window, cx);
                }
              }
            }
            Err(error) => {
              this.apps = Load::Failed(error.to_string().into());
              Self::error(window, cx, error.to_string());
            }
          }
          cx.notify();
        })
        .ok();
    }));
    cx.notify();
  }

  pub fn select_app(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
    self.show_settings = false;
    self.selected = Some(id);
    self.ai = AiState::Idle;
    self.report = None;
    self.load_snapshot(window, cx);
  }

  pub fn selected_app(&self) -> Option<StoreApp> {
    let Load::Ready(apps) = &self.apps else { return None };
    apps.iter().find(|app| Some(&app.id) == self.selected.as_ref()).cloned()
  }

  pub fn ready_snapshot(&self) -> Option<&AppSnapshot> {
    match &self.snapshot {
      Load::Ready(snapshot) => Some(snapshot),
      _ => None,
    }
  }

  pub fn privacy_declaration(&self) -> PrivacyDeclaration {
    self.selected.as_ref().and_then(|id| self.settings.privacy.get(id)).cloned().unwrap_or_default()
  }

  pub fn load_snapshot(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let (Some(client), Some(app)) = (self.client.clone(), self.selected_app()) else { return };
    self.snapshot = Load::Loading;
    self.tasks.snapshot = Some(cx.spawn_in(window, async move |this, cx| {
      let result = cx.background_spawn(async move { client.snapshot(&app) }).await;
      this
        .update_in(cx, |this, window, cx| {
          match result {
            Ok(snapshot) => this.snapshot_loaded(snapshot, window, cx),
            Err(error) => {
              this.snapshot = Load::Failed(error.to_string().into());
              Self::error(window, cx, error.to_string());
            }
          }
          cx.notify();
        })
        .ok();
    }));
    cx.notify();
  }

  fn snapshot_loaded(&mut self, snapshot: AppSnapshot, window: &mut Window, cx: &mut Context<Self>) {
    let available = locales(&snapshot);
    if !available.contains(&self.locale) {
      self.locale = available.first().cloned().unwrap_or_default();
    }
    self.forms.fill_locale(&snapshot, &self.locale, window, cx);
    self.forms.fill_review(snapshot.review_detail.as_ref(), window, cx);
    let declaration = self.privacy_declaration();
    set_input(&self.forms.data_types, &declaration.data_types, window, cx);
    set_input(&self.forms.sdks, &declaration.third_party_sdks, window, cx);
    self.snapshot = Load::Ready(snapshot);
    self.recompute_report();
  }

  pub fn select_locale(&mut self, locale: String, window: &mut Window, cx: &mut Context<Self>) {
    self.locale = locale;
    if let Load::Ready(snapshot) = &self.snapshot {
      let snapshot = snapshot.clone();
      self.forms.fill_locale(&snapshot, &self.locale, window, cx);
    }
    cx.notify();
  }

  pub fn recompute_report(&mut self) {
    let declaration = self.privacy_declaration();
    self.report = self.ready_snapshot().map(|snapshot| check(snapshot, &declaration));
  }

  // Saving ----------------------------------------------------------------------------------------------------

  fn run_save(
    &mut self,
    window: &mut Window,
    cx: &mut Context<Self>,
    work: impl FnOnce(&Client) -> Result<bool, appforge_appstore::Error> + Send + 'static,
  ) {
    let Some(client) = self.client.clone() else { return };
    if self.saving {
      return;
    }
    self.saving = true;
    self.tasks.save = Some(cx.spawn_in(window, async move |this, cx| {
      let result = cx.background_spawn(async move { work(&client) }).await;
      this
        .update_in(cx, |this, window, cx| {
          this.saving = false;
          match result {
            Ok(true) => {
              Self::success(window, cx, "Saved to App Store Connect.");
              this.load_snapshot(window, cx);
            }
            Ok(false) => Self::success(window, cx, "Nothing changed."),
            Err(error) => Self::error(window, cx, error.to_string()),
          }
          cx.notify();
        })
        .ok();
    }));
    cx.notify();
  }

  fn edited_info(&self, privacy_url: Option<String>, cx: &App) -> Option<(AppInfoLocalization, AppInfoLocalization)> {
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

  // Analysis --------------------------------------------------------------------------------------------------

  pub fn run_ai(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    if matches!(self.ai, AiState::Running) {
      return;
    }
    let (Some(snapshot), Some(report)) = (self.ready_snapshot().cloned(), self.report.clone()) else { return };
    let provider = self.settings.ai.provider;
    if !self.has_key.get(&provider).copied().unwrap_or(false) {
      Self::error(window, cx, format!("Add your {} API key in Settings first.", provider.label()));
      return;
    }
    let model = self.settings.ai.model(provider).to_owned();
    let declaration = self.privacy_declaration();
    let secrets = self.secrets.clone();
    let playbook = self.playbook.clone();
    let data_dir = self.data_dir.clone();
    self.ai = AiState::Running;
    self.tasks.ai = Some(cx.spawn_in(window, async move |this, cx| {
      let result = cx
        .background_spawn(async move {
          let key = secrets
            .get(SecretKey::ApiKey(provider))
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("Add your {} API key in Settings first.", provider.label()))?;
          // The rules are downloaded the first time they are needed.
          let playbook = if playbook.is_empty() { Playbook::refresh(&data_dir).unwrap_or(playbook) } else { playbook };
          let client = appforge_ai::Client::default();
          analyze_with_ai(&client, provider, &key, &model, &snapshot, &declaration, &report, &playbook).map(|review| (review, playbook))
        })
        .await;
      this
        .update_in(cx, |this, window, cx| {
          match result {
            Ok((review, playbook)) => {
              this.playbook = playbook;
              this.ai = AiState::Done(review);
            }
            Err(error) => {
              this.ai = AiState::Failed(error.clone().into());
              Self::error(window, cx, error);
            }
          }
          cx.notify();
        })
        .ok();
    }));
    cx.notify();
  }

  fn refresh_playbook_if_stale(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let stale = self.playbook.fetched_at.is_none_or(|fetched| now().saturating_sub(fetched) > PLAYBOOK_MAX_AGE_SECONDS);
    if stale {
      self.refresh_playbook(false, window, cx);
    }
  }

  /// Downloads the playbook. `announce` shows a notification with the outcome (when the user asked for it).
  pub fn refresh_playbook(&mut self, announce: bool, window: &mut Window, cx: &mut Context<Self>) {
    if self.playbook_busy {
      return;
    }
    let data_dir = self.data_dir.clone();
    self.playbook_busy = true;
    self.tasks.playbook = Some(cx.spawn_in(window, async move |this, cx| {
      let result = cx.background_spawn(async move { Playbook::refresh(&data_dir) }).await;
      this
        .update_in(cx, |this, window, cx| {
          this.playbook_busy = false;
          match result {
            Ok(playbook) => {
              this.playbook = playbook;
              this.playbook_error = None;
              if announce {
                Self::success(window, cx, "Rules updated from the playbook.");
              }
            }
            Err(error) => {
              this.playbook_error = Some(error.clone().into());
              if announce {
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

  // AI settings -----------------------------------------------------------------------------------------------

  pub fn set_provider(&mut self, provider: Provider, window: &mut Window, cx: &mut Context<Self>) {
    self.settings.ai.provider = provider;
    self.save_settings(window, cx);
    cx.notify();
  }

  /// Saves the model and, when one was typed, the API key of `provider`.
  pub fn save_ai(&mut self, provider: Provider, window: &mut Window, cx: &mut Context<Self>) {
    let (key_input, model_input) = match provider {
      Provider::OpenAi => (&self.settings_form.openai_key, &self.settings_form.openai_model),
      Provider::Anthropic => (&self.settings_form.anthropic_key, &self.settings_form.anthropic_model),
    };
    let key = read_input(key_input, cx);
    let model = read_input(model_input, cx);
    let model = if model.is_empty() { provider.default_model().to_owned() } else { model };
    match provider {
      Provider::OpenAi => self.settings.ai.openai_model = model,
      Provider::Anthropic => self.settings.ai.anthropic_model = model,
    }
    self.save_settings(window, cx);
    if key.is_empty() {
      Self::success(window, cx, format!("{} settings saved.", provider.label()));
      cx.notify();
      return;
    }
    let secrets = self.secrets.clone();
    self.tasks.secrets = Some(cx.spawn_in(window, async move |this, cx| {
      let result = cx.background_spawn(async move { secrets.set(SecretKey::ApiKey(provider), &key) }).await;
      this
        .update_in(cx, |this, window, cx| {
          match result {
            Ok(()) => {
              this.has_key.insert(provider, true);
              let input = match provider {
                Provider::OpenAi => this.settings_form.openai_key.clone(),
                Provider::Anthropic => this.settings_form.anthropic_key.clone(),
              };
              set_input(&input, "", window, cx);
              Self::success(window, cx, format!("{} API key saved in the Keychain.", provider.label()));
            }
            Err(error) => Self::error(window, cx, error.to_string()),
          }
          cx.notify();
        })
        .ok();
    }));
  }

  pub fn remove_key(&mut self, provider: Provider, window: &mut Window, cx: &mut Context<Self>) {
    let secrets = self.secrets.clone();
    self.tasks.secrets = Some(cx.spawn_in(window, async move |this, cx| {
      let result = cx.background_spawn(async move { secrets.delete(SecretKey::ApiKey(provider)) }).await;
      this
        .update_in(cx, |this, window, cx| {
          match result {
            Ok(()) => {
              this.has_key.insert(provider, false);
              Self::success(window, cx, format!("{} API key removed.", provider.label()));
            }
            Err(error) => Self::error(window, cx, error.to_string()),
          }
          cx.notify();
        })
        .ok();
    }));
  }
}
