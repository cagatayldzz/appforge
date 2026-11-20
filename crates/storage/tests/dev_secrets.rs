use appforge_ai::Provider;
use appforge_storage::{FileSecrets, SecretKey, SecretStore};
use std::os::unix::fs::PermissionsExt as _;

#[test]
fn secrets_survive_a_restart_and_stay_private_to_the_user() {
  let dir = tempfile::tempdir().unwrap();
  let path = dir.path().join("nested").join("dev-secrets.json");
  let store = FileSecrets::new(&path);
  assert_eq!(store.get(SecretKey::ApplePrivateKey).unwrap(), None);
  store.set(SecretKey::ApplePrivateKey, "pem").unwrap();
  store.set(SecretKey::ApiKey(Provider::Anthropic), "key").unwrap();
  let reopened = FileSecrets::new(&path);
  assert_eq!(reopened.get(SecretKey::ApplePrivateKey).unwrap().as_deref(), Some("pem"));
  assert_eq!(reopened.get(SecretKey::ApiKey(Provider::Anthropic)).unwrap().as_deref(), Some("key"));
  assert_eq!(std::fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
}

#[test]
fn deleting_a_secret_removes_only_that_secret() {
  let dir = tempfile::tempdir().unwrap();
  let store = FileSecrets::new(dir.path().join("dev-secrets.json"));
  store.set(SecretKey::ApplePrivateKey, "pem").unwrap();
  store.set(SecretKey::ApiKey(Provider::OpenAi), "key").unwrap();
  store.delete(SecretKey::ApplePrivateKey).unwrap();
  store.delete(SecretKey::ApplePrivateKey).unwrap();
  assert_eq!(store.get(SecretKey::ApplePrivateKey).unwrap(), None);
  assert_eq!(store.get(SecretKey::ApiKey(Provider::OpenAi)).unwrap().as_deref(), Some("key"));
}
