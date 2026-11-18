use appforge_storage::IconCache;
use std::{
  fs,
  sync::atomic::{AtomicUsize, Ordering},
};

fn temporary_dir(name: &str) -> std::path::PathBuf {
  static NEXT: AtomicUsize = AtomicUsize::new(0);
  let dir = std::env::temp_dir().join(format!("appforge-{name}-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
  fs::create_dir_all(&dir).unwrap();
  dir
}

const PNG: &[u8] = b"\x89PNG\r\n\x1a\nrest of the file";

#[test]
fn a_stored_icon_is_found_again_and_is_fresh() {
  let dir = temporary_dir("icons");
  let cache = IconCache::new(&dir);
  assert_eq!(cache.path("100"), None);
  assert!(!cache.is_fresh("100"));
  let stored = cache.store("100", PNG).unwrap();
  assert_eq!(cache.path("100"), Some(stored.clone()));
  assert_eq!(fs::read(stored).unwrap(), PNG);
  assert!(cache.is_fresh("100"));
  fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_file_that_is_not_an_image_or_an_unsafe_id_is_refused() {
  let dir = temporary_dir("icons-refused");
  let cache = IconCache::new(&dir);
  assert!(cache.store("100", b"<html>not an image</html>").is_err());
  assert!(cache.store("../escape", PNG).is_err());
  assert!(cache.store("", PNG).is_err());
  assert_eq!(cache.path("100"), None);
  assert!(!dir.join("icons").join("100.png.partial").exists());
  fs::remove_dir_all(dir).unwrap();
}
