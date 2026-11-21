//! The window's state: the session, the app list, the selected app and every background task.

use crate::{
  forms::{read_area, read_input, set_input, Forms},
  i18n::{tr, trf},
};
mod ai;
mod ai_dialog;
mod ai_progress;
mod apps;
mod form_history;
pub(crate) mod refresh;
mod save_preview;
mod saving;
mod session;
mod settings;
mod updates;

use appforge_ai::Provider;
use appforge_appstore::{App as StoreApp, AppInfoLocalization, AppSnapshot, Client, Credentials, ReviewDetail, VersionLocalization};
use appforge_compliance::{analyze_project_with_ai, check, AiReview, Guidelines, ListingField, Playbook, Report};
use appforge_storage::{AppleAccount, FileSecrets, IconCache, Keychain, Language, PrivacyDeclaration, SecretKey, SecretStore, Settings, UsageLedger};
use gpui_kit::{
  component::{
    input::InputState,
    notification::Notification,
    select::{SelectEvent, SelectState},
    IndexPath, WindowExt as _,
  },
  *,
};
use std::{
  collections::HashMap,
  path::PathBuf,
  sync::Arc,
  time::{Duration, SystemTime, UNIX_EPOCH},
};

/// The cached playbook is refreshed in the background when it is older than this.
const PLAYBOOK_MAX_AGE_SECONDS: u64 = 7 * 24 * 60 * 60;

/// The width, in pixels, of the icons asked for: they are drawn small, and twice that sharpens them on a Retina screen.
const ICON_SIZE: u32 = 128;

/// How long the window must be still before its place is written.
const WINDOW_SAVE_DELAY: Duration = Duration::from_millis(500);

#[derive(Default)]
pub enum Load<T> {
  #[default]
  Idle,
  Loading,
  Ready(T),
  Failed(SharedString),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DetailTab {
  ProductPage,
  Information,
  /// Pages that App Store Connect owns (the API has no endpoint for them): a short explanation and a link.
  Compliance,
  Distribution,
  /// The submissions sent to App Review, listed under App Review in the sidebar.
  Submissions,
  CustomerReviews,
  Pricing,
  Review,
  Privacy,
  Analysis,
}

impl DetailTab {
  pub const ALL: [DetailTab; 10] = [
    Self::Distribution,
    Self::ProductPage,
    Self::Information,
    Self::Compliance,
    Self::Submissions,
    Self::Review,
    Self::Privacy,
    Self::CustomerReviews,
    Self::Pricing,
    Self::Analysis,
  ];

  pub fn label(self) -> &'static str {
    match self {
      Self::ProductPage => "productPageInformation",
      Self::Distribution => "distribution",
      Self::CustomerReviews => "ratingsAndReviews",
      Self::Pricing => "pricingAndAvailability",
      Self::Information => "appInformation",
      Self::Compliance => "appCompliance",
      Self::Submissions => "appReview",
      Self::Review => "appReview",
      Self::Privacy => "appPrivacy",
      Self::Analysis => "analysis",
    }
  }
}

/// The pages of Settings, listed in the left column while Settings is open.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SettingsPage {
  General,
  Account,
  AiProvider,
  CodeFolders,
  Rules,
  About,
}

impl SettingsPage {
  pub const ALL: [SettingsPage; 6] = [Self::General, Self::Account, Self::AiProvider, Self::CodeFolders, Self::Rules, Self::About];

  pub fn label(self) -> &'static str {
    match self {
      Self::General => "general",
      Self::Account => "account",
      Self::AiProvider => "aiProvider",
      Self::CodeFolders => "codeFolders",
      Self::Rules => "rules",
      Self::About => "about",
    }
  }
}

pub enum AiState {
  Idle,
  Running(SharedString),
  Done(AiReview),
  Failed(SharedString),
}

pub struct LoginKey {
  pub file_name: String,
  pub pem: String,
}

pub struct LoginForm {
  /// The visible step of the sign-in guide: 0 create a key, 1 IDs, 2 private key.
  pub step: usize,
  pub issuer: Entity<InputState>,
  pub key_id: Entity<InputState>,
  pub key: Option<LoginKey>,
  /// The name field of the drawing of Apple's Generate API Key window.
  pub apple_name: Entity<InputState>,
  /// Which of the sub-steps of the first guide step the user has ticked off.
  pub done: [bool; crate::guide_art::KEY_STEP_COUNT],
  pub busy: bool,
  pub error: Option<SharedString>,
}

pub struct SettingsForm {
  pub openai_key: Entity<InputState>,
  pub anthropic_key: Entity<InputState>,
  /// The model menus: only the cheap models, each with its price.
  pub openai_model: Entity<SelectState<Vec<SharedString>>>,
  pub anthropic_model: Entity<SelectState<Vec<SharedString>>>,
}

/// Tasks live on the workspace; replacing one cancels the previous one.
#[derive(Default)]
pub(crate) struct Tasks {
  pub(crate) restore: Option<Task<()>>,
  pub(crate) sign_in: Option<Task<()>>,
  pub(crate) key_file: Option<Task<()>>,
  pub(crate) secrets: Option<Task<()>>,
  pub(crate) apps: Option<Task<()>>,
  pub(crate) save: Option<Task<()>>,
  pub(crate) form_history: Option<Task<()>>,
  pub(crate) ai: Option<Task<()>>,
  pub(crate) ai_dialog: Option<Task<()>>,
  pub(crate) playbook: Option<Task<()>>,
  pub(crate) guidelines: Option<Task<()>>,
  pub(crate) window: Option<Task<()>>,
  pub(crate) icons: Option<Task<()>>,
  pub(crate) versions: Option<Task<()>>,
  pub(crate) usage: Option<Task<()>>,
  pub(crate) folder: Option<Task<()>>,
  pub(crate) draft: Option<Task<()>>,
  pub(crate) fix: Option<Task<()>>,
  pub(crate) store_write: Option<Task<()>>,
}

#[derive(Default)]
pub(crate) struct AppData {
  pub snapshot: Load<AppSnapshot>,
  pub product_assets: HashMap<String, crate::product_page::MediaLoad>,
  pub distribution: Load<appforge_appstore::Distribution>,
  pub reviews: Load<Vec<appforge_appstore::CustomerReview>>,
  pub snapshot_task: Option<Task<()>>,
  pub distribution_task: Option<Task<()>>,
  pub review_version: Option<String>,
  pub review_versions: Option<Vec<(String, String)>>,
  pub review_territories: std::collections::BTreeSet<String>,
  pub reviews_task: Option<Task<()>>,
  pub pricing: Load<appforge_appstore::Pricing>,
  pub pricing_task: Option<Task<()>>,
}

pub struct Workspace {
  pub data_dir: PathBuf,
  updates: updates::Updates,
  pub settings: Settings,
  /// The settings file exists but could not be read: nothing is saved over it.
  pub settings_locked: bool,
  /// What the AI features have used, and whether its file could not be read (then it is never overwritten).
  pub(crate) usage: UsageLedger,
  pub(crate) usage_locked: bool,
  pub(crate) secrets: Arc<dyn SecretStore>,
  pub client: Option<Arc<Client>>,
  /// Reading the Keychain at startup.
  pub starting: bool,
  /// Shared, so a page that draws the apps takes a handle instead of copying every app.
  pub apps: Load<Arc<[StoreApp]>>,
  /// The icon file of each app, by app id, for the apps whose icon is known.
  pub icons: HashMap<String, PathBuf>,
  /// The version in front of each app, by app id, for the apps whose version is known.
  pub versions: HashMap<String, appforge_appstore::Version>,
  pub selected: Option<String>,
  pub(crate) app_data: HashMap<String, AppData>,
  pub(crate) store_write_busy: bool,
  pub tab: DetailTab,
  pub locale: String,
  /// Shared for the same reason as `apps`.
  pub report: Option<Arc<Report>>,
  pub ai: AiState,
  /// The page a jump from another page came from, so the new page can offer a way back. Picking a page in the sidebar clears it.
  /// The lists of the Distribution page that show all their rows instead of the first few.
  pub(crate) distribution_expanded: std::collections::HashSet<&'static str>,
  pub(crate) back_tab: Option<DetailTab>,
  pub(crate) analysis_tab: crate::analysis::AnalysisTab,
  /// The open part of App Information: 0 the fields to fill, 1 the display-only details.
  /// The open part of Product Page Information: 0 the listing texts, 1 the media.
  /// Findings whose open or closed state was flipped by a click (keys of `finding_row`).
  pub(crate) toggled_findings: std::collections::HashSet<String>,
  pub playbook: Playbook,
  pub(crate) guidelines: Arc<Guidelines>,
  pub(crate) guidelines_busy: bool,
  pub(crate) guidelines_error: Option<SharedString>,
  pub(crate) guideline_search: Entity<InputState>,
  pub(crate) guideline_section: Option<char>,
  pub playbook_busy: bool,
  pub playbook_error: Option<SharedString>,
  pub has_key: HashMap<Provider, bool>,
  pub show_settings: bool,
  pub settings_page: SettingsPage,
  /// The main area shows every app as a grid instead of one app.
  pub show_all_apps: bool,
  /// The filters of the customer reviews page.
  pub(crate) review_status: crate::customer_reviews::ReviewStatus,
  pub(crate) review_rating: Option<u64>,
  pub(crate) review_territory: Option<String>,
  pub(crate) review_sort: crate::customer_reviews::ReviewSort,
  pub saving: bool,
  pub login: LoginForm,
  /// The language menu shared by the welcome screen, the sign-in guide and Settings. Add a language to `Language::ALL` and it shows up here.
  pub language_select: Entity<SelectState<Vec<SharedString>>>,
  _subscriptions: Vec<Subscription>,
  pub forms: Forms,
  pub settings_form: SettingsForm,
  pub(crate) tasks: Tasks,
  /// The AI is writing the store listing.
  pub drafting: Option<ListingField>,
  /// The key of the finding the AI is fixing.
  pub(crate) fixing: Option<String>,
}

/// How a model shows in its menu: its name and what a million tokens cost, input / output.
pub(crate) fn model_label(info: &appforge_ai::ModelInfo) -> String {
  format!("{} · {}", info.label, info.price_text())
}

/// The menu of the models of `provider`, with the one in use chosen.
/// Release builds keep secrets in the Keychain; development builds use a private file, so the Keychain never asks for its password.
fn secret_store(data_dir: &std::path::Path) -> Arc<dyn SecretStore> {
  if cfg!(debug_assertions) {
    Arc::new(FileSecrets::new(data_dir.join("dev-secrets.json")))
  } else {
    Arc::new(Keychain)
  }
}

fn model_select(
  provider: Provider,
  ai: &appforge_storage::AiSettings,
  window: &mut Window,
  cx: &mut Context<Workspace>,
) -> Entity<SelectState<Vec<SharedString>>> {
  let labels: Vec<SharedString> = provider.models().iter().map(|info| SharedString::from(model_label(info))).collect();
  let chosen = ai.model(provider);
  let selected = provider.models().iter().position(|info| info.id == chosen).map(IndexPath::new);
  cx.new(|cx| SelectState::new(labels, selected, window, cx))
}

pub(crate) fn now() -> u64 {
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
    let (usage, usage_locked) = match UsageLedger::load(&data_dir) {
      Ok(usage) => (usage, false),
      Err(_) => (UsageLedger::default(), true),
    };
    let login = LoginForm {
      step: 0,
      issuer: cx.new(|cx| InputState::new(window, cx).placeholder("00000000-0000-0000-0000-000000000000")),
      key_id: cx.new(|cx| InputState::new(window, cx).placeholder("ABC123DEFG")),
      key: None,
      apple_name: cx.new(|cx| {
        let mut state = InputState::new(window, cx);
        state.set_value("AppForge", window, cx);
        state
      }),
      done: [false; crate::guide_art::KEY_STEP_COUNT],
      busy: false,
      error: None,
    };
    let settings_form = SettingsForm {
      openai_key: cx.new(|cx| InputState::new(window, cx).placeholder("sk-…").masked(true)),
      anthropic_key: cx.new(|cx| InputState::new(window, cx).placeholder("sk-ant-…").masked(true)),
      openai_model: model_select(Provider::OpenAi, &settings.ai, window, cx),
      anthropic_model: model_select(Provider::Anthropic, &settings.ai, window, cx),
    };
    let model_subscriptions: Vec<Subscription> =
      [(Provider::OpenAi, &settings_form.openai_model), (Provider::Anthropic, &settings_form.anthropic_model)]
        .into_iter()
        .map(|(provider, select)| {
          cx.subscribe_in(select, window, move |this, _, event: &SelectEvent<Vec<SharedString>>, window, cx| {
            let SelectEvent::Confirm(Some(label)) = event else { return };
            if let Some(info) = provider.models().iter().find(|info| model_label(info) == label.as_ref()) {
              this.set_model(provider, info.id, window, cx);
            }
          })
        })
        .collect();
    let forms = Forms::new(window, cx);
    let completion_subscriptions = [
      &forms.name,
      &forms.subtitle,
      &forms.marketing_url,
      &forms.privacy_choices,
      &forms.data_types,
      &forms.sdks,
      &forms.keywords,
      &forms.support_url,
      &forms.first_name,
      &forms.last_name,
      &forms.phone,
      &forms.email,
      &forms.demo_name,
      &forms.demo_password,
      &forms.privacy_url,
    ]
    .into_iter()
    .map(|field| cx.observe(field, |_, _, cx| cx.notify()))
    .collect::<Vec<_>>();
    let area_subscriptions = [&forms.description, &forms.promotional, &forms.whats_new, &forms.notes]
      .into_iter()
      .map(|field| cx.observe(field, |_, _, cx| cx.notify()))
      .collect::<Vec<_>>();
    let guideline_search = cx.new(|cx| InputState::new(window, cx));
    let guideline_search_subscription = cx.observe(&guideline_search, |_, _, cx| cx.notify());
    let language_names: Vec<SharedString> = Language::ALL.iter().map(|language| SharedString::new_static(language.native_name())).collect();
    let selected_language = Language::ALL.iter().position(|language| *language == settings.language).map(IndexPath::new);
    let language_select = cx.new(|cx| SelectState::new(language_names, selected_language, window, cx));
    let language_subscription = cx.subscribe_in(&language_select, window, |this, _, event: &SelectEvent<Vec<SharedString>>, window, cx| {
      let SelectEvent::Confirm(Some(name)) = event else { return };
      if let Some(language) = Language::ALL.iter().find(|language| language.native_name() == name.as_ref()) {
        this.set_language(*language, window, cx);
      }
    });
    let secrets = secret_store(&data_dir);
    let mut this = Self {
      data_dir,
      updates: updates::Updates::default(),
      settings,
      settings_locked,
      usage,
      usage_locked,
      secrets,
      client: None,
      starting: true,
      apps: Load::Idle,
      icons: HashMap::new(),
      versions: HashMap::new(),
      selected: None,
      app_data: HashMap::new(),
      store_write_busy: false,
      tab: DetailTab::ProductPage,
      locale: String::new(),
      report: None,
      ai: AiState::Idle,
      distribution_expanded: Default::default(),
      back_tab: None,
      analysis_tab: Default::default(),
      toggled_findings: Default::default(),
      playbook,
      guidelines: Arc::new(Guidelines::default()),
      guidelines_busy: false,
      guidelines_error: None,
      guideline_search,
      guideline_section: None,
      playbook_busy: false,
      playbook_error: None,
      has_key: HashMap::new(),
      show_settings: false,
      settings_page: SettingsPage::General,
      show_all_apps: false,
      review_status: Default::default(),
      review_rating: None,
      review_territory: None,
      review_sort: Default::default(),
      saving: false,
      login,
      language_select,
      _subscriptions: std::iter::once(language_subscription)
        .chain([
          cx.observe_window_appearance(window, |_, window, cx| crate::theme::apply_theme(window, cx)),
          cx.observe_window_bounds(window, |this, window, cx| this.remember_window(window, cx)),
          // Quitting right after a resize must not lose the new size: write it now instead of after the delay.
          cx.on_app_quit(|this, _| {
            this.save_window_now();
            async {}
          }),
        ])
        .chain(model_subscriptions)
        .chain(completion_subscriptions)
        .chain(area_subscriptions)
        .chain(std::iter::once(guideline_search_subscription))
        .collect(),
      forms,
      settings_form,
      tasks: Tasks::default(),
      drafting: None,
      fixing: None,
    };
    if this.settings_locked {
      this.login.error = Some(
        "The settings file could not be read, so nothing will be saved. Fix or remove settings.json in the data folder and restart AppForge.".into(),
      );
    }
    crate::theme::apply_theme(window, cx);
    this.restore(window, cx);
    if !this.settings.skip_update_check_on_launch {
      this.check_updates(false, window, cx);
    }
    this
  }

  pub(crate) fn error(window: &mut Window, cx: &mut App, message: impl Into<SharedString>) {
    window.push_notification(Notification::error(message), cx);
  }

  pub(crate) fn warning(window: &mut Window, cx: &mut App, message: impl Into<SharedString>) {
    window.push_notification(Notification::warning(message), cx);
  }

  pub(crate) fn success(window: &mut Window, cx: &mut App, message: impl Into<SharedString>) {
    window.push_notification(Notification::success(message), cx);
  }
}
