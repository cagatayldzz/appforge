use appforge_project::summarize;
use std::{
  fs,
  path::{Path, PathBuf},
  sync::atomic::{AtomicUsize, Ordering},
};

fn temporary_dir(name: &str) -> PathBuf {
  static NEXT: AtomicUsize = AtomicUsize::new(0);
  let dir = std::env::temp_dir().join(format!("appforge-{name}-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
  fs::create_dir_all(&dir).unwrap();
  dir
}

fn write(root: &Path, path: &str, content: &str) {
  let file = root.join(path);
  fs::create_dir_all(file.parent().unwrap()).unwrap();
  fs::write(file, content).unwrap();
}

#[test]
fn the_summary_has_the_structure_the_readme_and_the_manifests() {
  let root = temporary_dir("summary");
  write(&root, "README.md", "Hexumi is a word puzzle.");
  write(&root, "Sources/App/GameView.swift", "struct GameView {}");
  write(&root, "Sources/App/en.lproj/Localizable.strings", "\"play\" = \"Play\";");
  write(&root, "Package.swift", "// swift-tools-version: 5.9");
  let summary = summarize(&root).unwrap();
  assert!(summary.text.contains("Sources/"), "{}", summary.text);
  assert!(summary.text.contains("GameView.swift"));
  assert!(summary.text.contains("Hexumi is a word puzzle."));
  assert!(summary.text.contains("swift-tools-version"));
  assert!(summary.text.contains("\"play\" = \"Play\""));
  assert!(!summary.truncated);
  fs::remove_dir_all(root).unwrap();
}

#[test]
fn secrets_other_peoples_code_and_hidden_folders_are_left_out() {
  let root = temporary_dir("secrets");
  write(&root, "README.md", "readme");
  write(&root, ".env", "API_KEY=sk-live-123");
  write(&root, "AuthKey_ABC123DEFG.p8", "-----BEGIN PRIVATE KEY-----");
  write(&root, "GoogleService-Info.plist", "<key>API_KEY</key>");
  write(&root, "config/secrets.json", "{\"token\":\"abc\"}");
  write(&root, "node_modules/lib/package.json", "{\"name\":\"other\"}");
  write(&root, ".git/config", "[core]");
  let text = summarize(&root).unwrap().text;
  for hidden in ["sk-live-123", "BEGIN PRIVATE KEY", "AuthKey_", "GoogleService", "secrets.json", "token", "node_modules", "other", ".git"] {
    assert!(!text.contains(hidden), "{hidden} leaked into: {text}");
  }
  assert!(text.contains("readme"));
  fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn a_link_that_leaves_the_project_is_not_followed() {
  let outside = temporary_dir("outside");
  write(&outside, "README.md", "OUTSIDE CONTENT");
  let root = temporary_dir("links");
  write(&root, "README.md", "inside");
  std::os::unix::fs::symlink(&outside, root.join("linked")).unwrap();
  std::os::unix::fs::symlink(outside.join("README.md"), root.join("package.json")).unwrap();
  let text = summarize(&root).unwrap().text;
  assert!(!text.contains("OUTSIDE CONTENT") && !text.contains("linked"), "{text}");
  fs::remove_dir_all(root).unwrap();
  fs::remove_dir_all(outside).unwrap();
}

#[test]
fn a_big_folder_is_cut_to_a_size_that_fits_a_prompt() {
  let root = temporary_dir("big");
  write(&root, "README.md", &"word ".repeat(10_000));
  write(&root, "package.json", &"{\"a\":1}".repeat(3_000));
  write(&root, "docs/README.md", &"more ".repeat(10_000));
  for index in 0..300 {
    write(&root, &format!("files/file{index}.txt"), "x");
  }
  let summary = summarize(&root).unwrap();
  assert!(summary.text.chars().count() <= 41_000, "{}", summary.text.chars().count());
  assert!(summary.truncated);
  fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_missing_folder_or_a_file_is_an_error() {
  let root = temporary_dir("missing");
  assert!(summarize(&root.join("nothing")).is_err());
  write(&root, "file.txt", "x");
  assert!(summarize(&root.join("file.txt")).is_err());
  fs::remove_dir_all(root).unwrap();
}
