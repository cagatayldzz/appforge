//! The Keychain (`Keychain`) and an in-memory store for tests.

use appforge_ai::Provider;
use std::{collections::HashMap, fmt, sync::Mutex};

const SERVICE: &str = "AppForge";

/// A secret AppForge keeps in the Keychain.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SecretKey {
  /// The contents of the App Store Connect `.p8` private key.
  ApplePrivateKey,
  ApiKey(Provider),
  HistoryPassword(git2::Oid, bool),
}

impl SecretKey {
  fn account(self) -> String {
    match self {
      Self::ApplePrivateKey => "apple-private-key".into(),
      Self::ApiKey(Provider::OpenAi) => "openai-api-key".into(),
      Self::ApiKey(Provider::Anthropic) => "anthropic-api-key".into(),
      Self::HistoryPassword(id, before) => format!("history-password-{id}-{before}"),
    }
  }
}

#[derive(Debug)]
pub struct SecretError(String);

impl fmt::Display for SecretError {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(formatter, "Keychain error: {}", self.0)
  }
}

impl std::error::Error for SecretError {}

pub trait SecretStore: Send + Sync {
  fn get(&self, key: SecretKey) -> Result<Option<String>, SecretError>;
  fn set(&self, key: SecretKey, value: &str) -> Result<(), SecretError>;
  fn delete(&self, key: SecretKey) -> Result<(), SecretError>;
}

/// The macOS Keychain.
#[derive(Clone, Copy, Debug, Default)]
pub struct Keychain;

fn entry(key: SecretKey) -> Result<keyring::Entry, SecretError> {
  keyring::Entry::new(SERVICE, &key.account()).map_err(|error| SecretError(error.to_string()))
}

impl SecretStore for Keychain {
  fn get(&self, key: SecretKey) -> Result<Option<String>, SecretError> {
    match entry(key)?.get_password() {
      Ok(value) => Ok(Some(value)),
      Err(keyring::Error::NoEntry) => Ok(None),
      Err(error) => Err(SecretError(error.to_string())),
    }
  }

  fn set(&self, key: SecretKey, value: &str) -> Result<(), SecretError> {
    entry(key)?.set_password(value).map_err(|error| SecretError(error.to_string()))
  }

  fn delete(&self, key: SecretKey) -> Result<(), SecretError> {
    match entry(key)?.delete_credential() {
      Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
      Err(error) => Err(SecretError(error.to_string())),
    }
  }
}

/// Secrets held in memory only; for tests.
#[derive(Debug, Default)]
pub struct MemorySecrets(Mutex<HashMap<SecretKey, String>>);

impl SecretStore for MemorySecrets {
  fn get(&self, key: SecretKey) -> Result<Option<String>, SecretError> {
    Ok(self.0.lock().map_err(|error| SecretError(error.to_string()))?.get(&key).cloned())
  }

  fn set(&self, key: SecretKey, value: &str) -> Result<(), SecretError> {
    self.0.lock().map_err(|error| SecretError(error.to_string()))?.insert(key, value.to_owned());
    Ok(())
  }

  fn delete(&self, key: SecretKey) -> Result<(), SecretError> {
    self.0.lock().map_err(|error| SecretError(error.to_string()))?.remove(&key);
    Ok(())
  }
}
