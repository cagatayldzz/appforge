//! Persistent state for AppForge: `settings.json` under `~/Library/Application Support/AppForge`
//! and secrets (the App Store Connect private key, AI API keys) in the macOS Keychain.

mod history;
mod icons;
mod paths;
mod secrets;
mod settings;
mod usage;

pub use history::{History, HistoryRevision, HistoryValues};
pub use icons::IconCache;
pub use paths::data_dir;
pub use secrets::{Keychain, MemorySecrets, SecretError, SecretKey, SecretStore};
pub use settings::{AiSettings, AppleAccount, Language, PrivacyDeclaration, Settings, WindowMode, WindowState};
pub use usage::{ModelUsage, UsageLedger, UsageTotals, USAGE_FILE};
