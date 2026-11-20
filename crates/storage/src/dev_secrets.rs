//! Secrets in a private file, for development builds: an ad-hoc signed build is new on every build, so the Keychain would ask for its
//! password each time. Release builds use the Keychain.

use crate::secrets::{SecretError, SecretKey, SecretStore};
use std::{
  collections::BTreeMap,
  fs,
  io::Write as _,
  os::unix::fs::OpenOptionsExt as _,
  path::{Path, PathBuf},
  sync::Mutex,
};

/// A JSON file that only the current user can read.
#[derive(Debug)]
pub struct FileSecrets {
  path: PathBuf,
  lock: Mutex<()>,
}

impl FileSecrets {
  pub fn new(path: impl Into<PathBuf>) -> Self {
    Self { path: path.into(), lock: Mutex::new(()) }
  }

  fn read(&self) -> Result<BTreeMap<String, String>, SecretError> {
    match fs::read(&self.path) {
      Ok(bytes) => serde_json::from_slice(&bytes).map_err(|error| SecretError::new(error.to_string())),
      Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(BTreeMap::new()),
      Err(error) => Err(SecretError::new(error.to_string())),
    }
  }

  fn write(&self, secrets: &BTreeMap<String, String>) -> Result<(), SecretError> {
    let error = |error: &dyn std::fmt::Display| SecretError::new(error.to_string());
    if let Some(parent) = self.path.parent() {
      fs::create_dir_all(parent).map_err(|e| error(&e))?;
    }
    let temporary = temporary_path(&self.path);
    let bytes = serde_json::to_vec_pretty(secrets).map_err(|e| error(&e))?;
    let mut file = fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(&temporary).map_err(|e| error(&e))?;
    file.write_all(&bytes).map_err(|e| error(&e))?;
    fs::rename(&temporary, &self.path).map_err(|e| error(&e))
  }
}

fn temporary_path(path: &Path) -> PathBuf {
  path.with_extension("json.tmp")
}

impl SecretStore for FileSecrets {
  fn get(&self, key: SecretKey) -> Result<Option<String>, SecretError> {
    let _guard = self.lock.lock().map_err(|e| SecretError::new(e.to_string()))?;
    Ok(self.read()?.get(&key.account()).cloned())
  }

  fn set(&self, key: SecretKey, value: &str) -> Result<(), SecretError> {
    let _guard = self.lock.lock().map_err(|e| SecretError::new(e.to_string()))?;
    let mut secrets = self.read()?;
    secrets.insert(key.account(), value.to_owned());
    self.write(&secrets)
  }

  fn delete(&self, key: SecretKey) -> Result<(), SecretError> {
    let _guard = self.lock.lock().map_err(|e| SecretError::new(e.to_string()))?;
    let mut secrets = self.read()?;
    if secrets.remove(&key.account()).is_some() {
      self.write(&secrets)?;
    }
    Ok(())
  }
}
