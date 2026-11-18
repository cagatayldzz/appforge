use appforge_ai::Provider;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, io, path::Path};

/// The App Store Connect API key that is signed in. The private key itself lives in the Keychain.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct AppleAccount {
  pub issuer_id: String,
  pub key_id: String,
}

/// Which AI provider analyses an app, and the model each provider uses. API keys live in the Keychain.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct AiSettings {
  pub provider: Provider,
  pub openai_model: String,
  pub anthropic_model: String,
}

impl Default for AiSettings {
  fn default() -> Self {
    Self {
      provider: Provider::Anthropic,
      openai_model: Provider::OpenAi.default_model().to_owned(),
      anthropic_model: Provider::Anthropic.default_model().to_owned(),
    }
  }
}

impl AiSettings {
  pub fn model(&self, provider: Provider) -> &str {
    match provider {
      Provider::OpenAi => &self.openai_model,
      Provider::Anthropic => &self.anthropic_model,
    }
  }
}

/// What App Store Connect's API cannot tell us about an app: the answers behind its App Privacy
/// declaration and its review setup. The user fills these in; the analysis uses them.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct PrivacyDeclaration {
  /// The user confirmed that App Privacy is answered in App Store Connect.
  pub confirmed_in_app_store_connect: bool,
  pub collects_data: bool,
  pub uses_tracking: bool,
  pub shows_tracking_prompt: bool,
  pub has_accounts: bool,
  pub account_deletion_in_app: bool,
  pub requires_login_to_review: bool,
  /// Third-party SDKs, comma separated (for example "Firebase, AdMob").
  pub third_party_sdks: String,
  /// Data types the app collects, free text (for example "email, purchases, crash logs").
  pub data_types: String,
}

/// The language of the AppForge interface.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
  #[default]
  English,
  Turkish,
}

impl Language {
  pub const ALL: [Language; 2] = [Language::English, Language::Turkish];

  /// The name of the language in that language, so people find their own.
  pub fn native_name(self) -> &'static str {
    match self {
      Language::English => "English",
      Language::Turkish => "Türkçe",
    }
  }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct Settings {
  /// The welcome screen was shown and finished.
  pub onboarded: bool,
  pub language: Language,
  pub apple: Option<AppleAccount>,
  pub ai: AiSettings,
  /// Privacy answers per App Store Connect app id.
  pub privacy: BTreeMap<String, PrivacyDeclaration>,
}

impl Settings {
  /// A missing file is the default settings; an unreadable one is an error so it is never overwritten silently.
  pub fn load(path: &Path) -> io::Result<Self> {
    match fs::read(path) {
      Ok(data) => serde_json::from_slice(&data).map_err(io::Error::other),
      Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
      Err(error) => Err(error),
    }
  }

  pub fn save(&self, path: &Path) -> io::Result<()> {
    let parent = path.parent().ok_or_else(|| io::Error::other("Missing data directory"))?;
    fs::create_dir_all(parent)?;
    let data = serde_json::to_vec_pretty(self).map_err(io::Error::other)?;
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, data)?;
    fs::rename(temporary, path)
  }
}
