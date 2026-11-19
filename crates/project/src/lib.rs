//! A short description of an app's source folder, small enough to send to an AI: the shape of the folder, the README,
//! the manifests that name the app and what it uses, and the texts the app shows. Secrets are never read.

use std::{
  fs, io,
  path::{Path, PathBuf},
  process::Command,
};

/// The most characters of the whole description. A character is about a quarter of a token, so this is about 10,000 tokens.
const TOTAL_LIMIT: usize = 40_000;
/// The most characters taken from one file.
const FILE_LIMIT: usize = 6_000;
/// The most entries listed in the folder tree.
const TREE_LIMIT: usize = 150;
/// How deep the folder tree goes.
const TREE_DEPTH: usize = 3;
/// How many files are looked at before the search stops, so that a huge folder is never walked to its end.
const VISIT_LIMIT: usize = 20_000;

/// The most recent commit subjects listed, and the most characters they take.
const COMMIT_LIMIT: usize = 40;
const COMMITS_TEXT_LIMIT: usize = 3_000;

/// Folders that hold other people's code, builds or caches.
const SKIPPED_FOLDERS: &[&str] = &[
  "node_modules",
  "target",
  "build",
  "dist",
  "pods",
  "deriveddata",
  ".build",
  "venv",
  "__pycache__",
  "vendor",
  "carthage",
  "out",
  "coverage",
  "checkouts",
];

/// Files that hold keys, passwords or signing material; their content is never read, and their names are not listed.
const SECRET_NAMES: &[&str] =
  &[".env", "googleservice-info.plist", "google-services.json", "id_rsa", "id_ed25519", "secrets.json", "credentials.json", "serviceaccount.json"];
const SECRET_EXTENSIONS: &[&str] = &["p8", "p12", "pem", "key", "keystore", "jks", "mobileprovision", "cer", "pfx", "kdbx"];

/// File names that say what the app is and does, most useful first.
const KEY_FILES: &[&str] = &[
  "readme.md",
  "readme",
  "readme.txt",
  "package.swift",
  "info.plist",
  "package.json",
  "pubspec.yaml",
  "cargo.toml",
  "androidmanifest.xml",
  "podfile",
  "build.gradle",
  "build.gradle.kts",
  "project.yml",
  "app.json",
];

/// What was found in the folder.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProjectSummary {
  /// The description, ready to put in a prompt.
  pub text: String,
  /// How many files were looked at.
  pub files_seen: usize,
  /// Whether the folder had more than fits.
  pub truncated: bool,
}

fn lower_name(path: &Path) -> String {
  path.file_name().map(|name| name.to_string_lossy().to_lowercase()).unwrap_or_default()
}

fn is_secret(path: &Path) -> bool {
  let name = lower_name(path);
  if SECRET_NAMES.contains(&name.as_str())
    || name.starts_with(".env")
    || name.contains("secret")
    || name.contains("credential")
    || name.contains("private")
  {
    return true;
  }
  path.extension().is_some_and(|extension| SECRET_EXTENSIONS.contains(&extension.to_string_lossy().to_lowercase().as_str()))
}

fn is_skipped_folder(name: &str) -> bool {
  let lower = name.to_lowercase();
  lower.starts_with('.') || SKIPPED_FOLDERS.contains(&lower.as_str())
}

/// The entries of a folder, sorted by name, without links (a link could lead out of the project), secrets and skipped folders.
fn entries(dir: &Path) -> Vec<(PathBuf, bool)> {
  let Ok(read) = fs::read_dir(dir) else { return Vec::new() };
  let mut found: Vec<(PathBuf, bool)> = read
    .filter_map(Result::ok)
    .filter_map(|entry| {
      let kind = entry.file_type().ok()?;
      let path = entry.path();
      if kind.is_symlink() || is_secret(&path) {
        return None;
      }
      if kind.is_dir() {
        let name = entry.file_name().to_string_lossy().to_string();
        return (!is_skipped_folder(&name)).then_some((path, true));
      }
      kind.is_file().then_some((path, false))
    })
    .collect();
  found.sort();
  found
}

/// The first `limit` characters of a text file; `None` for a file that is not text.
fn read_text(path: &Path, limit: usize) -> Option<String> {
  let bytes = fs::read(path).ok()?;
  let text = String::from_utf8(bytes).ok()?;
  Some(text.chars().take(limit).collect())
}

fn is_text_resource(path: &Path) -> bool {
  let name = lower_name(path);
  name.ends_with(".strings") || name.ends_with(".xcstrings") || name == "strings.xml" || name.ends_with(".arb")
}

struct Walk {
  tree: Vec<String>,
  key_files: Vec<PathBuf>,
  resources: Vec<PathBuf>,
  visited: usize,
  truncated: bool,
}

fn walk(dir: &Path, root: &Path, depth: usize, state: &mut Walk) {
  for (path, is_dir) in entries(dir) {
    state.visited += 1;
    if state.visited > VISIT_LIMIT {
      state.truncated = true;
      return;
    }
    let relative = path.strip_prefix(root).unwrap_or(&path).to_string_lossy().to_string();
    if depth < TREE_DEPTH {
      if state.tree.len() < TREE_LIMIT {
        state.tree.push(if is_dir { format!("{relative}/") } else { relative.clone() });
      } else {
        state.truncated = true;
      }
    }
    if is_dir {
      walk(&path, root, depth + 1, state);
    } else if KEY_FILES.contains(&lower_name(&path).as_str()) {
      state.key_files.push(path);
    } else if is_text_resource(&path) {
      state.resources.push(path);
    }
  }
}

/// What changed lately, as the subjects of the newest commits (since the last tag when there is one). This is what release
/// notes can be written from. Read only; `None` when the folder is not a repository or Git is not installed.
fn recent_changes(root: &Path) -> Option<String> {
  let git = |args: &[&str]| -> Option<String> {
    let output = Command::new("git").arg("-C").arg(root).args(args).env("GIT_TERMINAL_PROMPT", "0").output().ok()?;
    output.status.success().then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
  };
  let limit = format!("--max-count={COMMIT_LIMIT}");
  let since_tag = git(&["describe", "--tags", "--abbrev=0"]).filter(|tag| !tag.is_empty()).map(|tag| format!("{tag}..HEAD"));
  let mut log = vec!["log", "--no-merges", "--pretty=format:- %s", limit.as_str()];
  if let Some(range) = since_tag.as_deref() {
    log.push(range);
  }
  let mut subjects = git(&log).filter(|text| !text.is_empty());
  if subjects.is_none() && since_tag.is_some() {
    // Nothing since the last tag: the newest commits are still the best evidence there is.
    subjects = git(&["log", "--no-merges", "--pretty=format:- %s", limit.as_str()]).filter(|text| !text.is_empty());
  }
  subjects.map(|text| text.chars().take(COMMITS_TEXT_LIMIT).collect())
}

/// Describes the folder `root`. Fails when it is not a folder that can be read.
pub fn summarize(root: &Path) -> io::Result<ProjectSummary> {
  summarize_source(root, true)
}

/// Summarizes review-relevant source context without reading Git history.
pub fn summarize_for_review(root: &Path) -> io::Result<ProjectSummary> {
  summarize_source(root, false)
}

fn summarize_source(root: &Path, include_git: bool) -> io::Result<ProjectSummary> {
  let metadata = fs::metadata(root)?;
  if !metadata.is_dir() {
    return Err(io::Error::new(io::ErrorKind::InvalidInput, "the project path is not a folder"));
  }
  let mut state = Walk { tree: Vec::new(), key_files: Vec::new(), resources: Vec::new(), visited: 0, truncated: false };
  walk(root, root, 0, &mut state);
  // The README and the manifests nearest to the top say the most, so they come first.
  let rank = |path: &PathBuf| {
    let name = lower_name(path);
    (path.components().count(), KEY_FILES.iter().position(|key| *key == name).unwrap_or(KEY_FILES.len()))
  };
  state.key_files.sort_by_key(rank);
  state.resources.sort_by_key(|path| path.components().count());

  let mut text = String::new();
  text.push_str("FOLDER STRUCTURE (top levels):\n");
  for line in &state.tree {
    text.push_str(line);
    text.push('\n');
  }
  if let Some(changes) = include_git.then(|| recent_changes(root)).flatten() {
    text.push_str(&format!("\n--- RECENT CHANGES (git commit subjects, newest first) ---\n{changes}\n"));
  }
  let mut truncated = state.truncated;
  let sections = state
    .key_files
    .iter()
    .map(|path| ("FILE", path, FILE_LIMIT))
    .chain(state.resources.iter().take(4).map(|path| ("TEXTS THE APP SHOWS", path, 3_000)));
  for (heading, path, limit) in sections {
    let Some(content) = read_text(path, limit) else { continue };
    let relative = path.strip_prefix(root).unwrap_or(path).to_string_lossy().to_string();
    let block = format!("\n--- {heading}: {relative} ---\n{content}\n");
    if text.chars().count() + block.chars().count() > TOTAL_LIMIT {
      truncated = true;
      break;
    }
    text.push_str(&block);
  }
  Ok(ProjectSummary { text, files_seen: state.visited, truncated })
}

/// Runs AppForge's local checklist against a bounded source summary.
pub fn quick_check(root: &Path) -> io::Result<Vec<String>> {
  use std::{io::Write as _, process::Stdio};

  let summary = summarize_for_review(root)?;
  let script = include_str!("../../../scripts/project-check.sh");
  let mut child = Command::new("/bin/sh").arg("-c").arg(script).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn()?;
  if let Some(mut stdin) = child.stdin.take() {
    stdin.write_all(summary.text.as_bytes())?;
  }
  let output = child.wait_with_output()?;
  if !output.status.success() {
    return Err(io::Error::other("The project checklist script failed."));
  }
  Ok(String::from_utf8_lossy(&output.stdout).lines().map(str::to_owned).collect())
}
