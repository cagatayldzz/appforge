use appforge_ai::Provider;
use appforge_storage::{AppleAccount, Language, MemorySecrets, PrivacyDeclaration, SecretKey, SecretStore, Settings, WindowMode, WindowState};
use std::{
  fs,
  sync::atomic::{AtomicUsize, Ordering},
};

fn temporary_dir(name: &str) -> std::path::PathBuf {
  // The counter keeps parallel tests apart: the clock alone can return the same value twice.
  static NEXT: AtomicUsize = AtomicUsize::new(0);
  let dir = std::env::temp_dir().join(format!("appforge-{name}-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
  fs::create_dir_all(&dir).unwrap();
  dir
}

#[test]
fn settings_round_trip_and_a_missing_file_is_the_default() {
  let dir = temporary_dir("settings");
  let path = dir.join("nested").join("settings.json");
  assert_eq!(Settings::load(&path).unwrap(), Settings::default());
  let mut settings = Settings { apple: Some(AppleAccount { issuer_id: "issuer".into(), key_id: "KEY123".into() }), ..Default::default() };
  settings.ai.provider = Provider::OpenAi;
  settings.privacy.insert("42".into(), PrivacyDeclaration { collects_data: true, ..Default::default() });
  settings.save(&path).unwrap();
  assert_eq!(Settings::load(&path).unwrap(), settings);
  fs::remove_dir_all(dir).unwrap();
}

#[test]
fn an_unreadable_settings_file_is_an_error_not_the_default() {
  let dir = temporary_dir("settings-bad");
  let path = dir.join("settings.json");
  fs::write(&path, "not json").unwrap();
  assert!(Settings::load(&path).is_err());
  fs::remove_dir_all(dir).unwrap();
}

#[test]
fn missing_fields_fall_back_to_defaults() {
  let dir = temporary_dir("settings-partial");
  let path = dir.join("settings.json");
  fs::write(&path, r#"{"apple":{"issuer_id":"a","key_id":"b"}}"#).unwrap();
  let settings = Settings::load(&path).unwrap();
  assert_eq!(settings.ai.provider, Provider::Anthropic);
  assert!(settings.privacy.is_empty());
  fs::remove_dir_all(dir).unwrap();
}

#[test]
fn memory_secrets_store_read_and_delete() {
  let secrets = MemorySecrets::default();
  assert_eq!(secrets.get(SecretKey::ApplePrivateKey).unwrap(), None);
  secrets.set(SecretKey::ApiKey(Provider::OpenAi), "sk-test").unwrap();
  assert_eq!(secrets.get(SecretKey::ApiKey(Provider::OpenAi)).unwrap().as_deref(), Some("sk-test"));
  assert_eq!(secrets.get(SecretKey::ApiKey(Provider::Anthropic)).unwrap(), None);
  secrets.delete(SecretKey::ApiKey(Provider::OpenAi)).unwrap();
  assert_eq!(secrets.get(SecretKey::ApiKey(Provider::OpenAi)).unwrap(), None);
}

#[test]
fn language_and_onboarding_default_then_round_trip() {
  let dir = temporary_dir("settings-language");
  let path = dir.join("settings.json");
  fs::write(&path, "{}").unwrap();
  let loaded = Settings::load(&path).unwrap();
  assert_eq!(loaded.language, Language::English);
  assert!(!loaded.onboarded);
  let settings = Settings { language: Language::Turkish, onboarded: true, ..Default::default() };
  settings.save(&path).unwrap();
  assert!(fs::read_to_string(&path).unwrap().contains("\"turkish\""));
  assert_eq!(Settings::load(&path).unwrap(), settings);
  fs::remove_dir_all(dir).unwrap();
}

#[test]
fn the_window_state_is_kept_and_old_settings_without_it_still_load() {
  let dir = temporary_dir("window");
  let path = dir.join("settings.json");
  fs::write(&path, r#"{"onboarded":true}"#).unwrap();
  assert_eq!(Settings::load(&path).unwrap().window, None);
  let settings = Settings { window: Some(WindowState { mode: WindowMode::Maximized, x: 40, y: 60, width: 1200, height: 800 }), ..Default::default() };
  settings.save(&path).unwrap();
  assert_eq!(Settings::load(&path).unwrap().window, settings.window);
  fs::remove_dir_all(dir).unwrap();
}

#[test]
fn code_folders_are_kept_per_app_and_old_settings_have_none() {
  let dir = temporary_dir("code-folders");
  let path = dir.join("settings.json");
  fs::write(&path, r#"{"onboarded":true}"#).unwrap();
  assert!(Settings::load(&path).unwrap().code_folders.is_empty());
  let mut settings = Settings::default();
  settings.code_folders.insert("100".into(), "/projects/hexumi".into());
  settings.save(&path).unwrap();
  assert_eq!(Settings::load(&path).unwrap().code_folders.get("100"), Some(&std::path::PathBuf::from("/projects/hexumi")));
  fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_model_that_is_no_longer_offered_is_replaced_by_the_default() {
  let mut settings = Settings::default();
  settings.ai.anthropic_model = "claude-sonnet-5-5".into();
  settings.ai.openai_model = "gpt-4.1-mini".into();
  assert_eq!(settings.ai.model(Provider::Anthropic), Provider::Anthropic.default_model());
  assert_eq!(settings.ai.model(Provider::OpenAi), "gpt-4.1-mini");
}
