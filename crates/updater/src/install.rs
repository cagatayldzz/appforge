//! Verification, staging and replacement of the installed macOS bundle.

use crate::{
  process::{download, get, run},
  release::checksum_for,
  Release, UpdateError,
};
use std::{
  fs,
  os::unix::process::CommandExt,
  path::{Path, PathBuf},
  process::{Command, Stdio},
};

const APP_NAME: &str = "AppForge.app";
const BUNDLE_ID: &str = "com.cagatayldzz.appforge";

/// The steps of preparing an update, reported to the interface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
  Downloading,
  Verifying,
}

/// A verified replacement app waiting to be swapped in.
#[derive(Debug)]
pub struct Staged {
  app: PathBuf,
  directory: tempfile::TempDir,
}

/// The `.app` bundle the running executable lives in.
pub fn current_app_bundle() -> Result<PathBuf, UpdateError> {
  let exe = std::env::current_exe().map_err(|e| UpdateError::new(format!("Could not locate AppForge: {e}")))?;
  let app = bundle_of(&exe).ok_or_else(|| UpdateError::new("Updates require an installed AppForge release."))?;
  if bundle_value(&app, "CFBundleIdentifier")?.trim() != BUNDLE_ID || signing_team(&app)?.is_none() {
    return Err(UpdateError::new("Updates require an installed, signed AppForge release."));
  }
  Ok(app)
}

/// The `.app` bundle an executable at `…/AppForge.app/Contents/MacOS/AppForge` lives in; `None` for any other location.
pub fn bundle_of(executable: &Path) -> Option<PathBuf> {
  let bundle = executable.parent()?.parent()?.parent()?;
  (bundle.extension()? == "app" && executable.parent()?.file_name()? == "MacOS").then(|| bundle.to_path_buf())
}

/// Downloads the release next to the running app, verifies it, and returns the
/// replacement. Staging beside the app keeps the final swap an atomic rename.
pub fn prepare(release: &Release, current_app: &Path, mut on_stage: impl FnMut(Stage)) -> Result<Staged, UpdateError> {
  let parent = current_app.parent().ok_or_else(|| UpdateError::new("Could not locate the AppForge folder."))?;
  let directory = tempfile::Builder::new()
    .prefix(".appforge-update-")
    .tempdir_in(parent)
    .map_err(|_| UpdateError::new("AppForge cannot write beside the installed app. Move it to a writable Applications folder."))?;
  let app = stage(release, current_app, directory.path(), &mut on_stage)?;
  Ok(Staged { app, directory })
}

fn stage(release: &Release, current_app: &Path, directory: &Path, on_stage: &mut impl FnMut(Stage)) -> Result<PathBuf, UpdateError> {
  on_stage(Stage::Downloading);
  let archive = directory.join(&release.archive_name);
  download(&release.archive_url, &archive)?;
  let (status, listing) = get(&release.checksums_url)?;
  if status != 200 {
    return Err(UpdateError::new("Could not download the release checksums."));
  }

  on_stage(Stage::Verifying);
  let expected = checksum_for(&listing, &release.archive_name).ok_or_else(|| UpdateError::new("The release checksums do not list the download."))?;
  let actual = run("/usr/bin/shasum", &["-a", "256", &archive.to_string_lossy()])?;
  if actual.split_whitespace().next().map(str::to_ascii_lowercase).as_deref() != Some(expected.as_str()) {
    return Err(UpdateError::new("The download does not match its published checksum."));
  }
  let extracted = directory.join("extracted");
  run("/usr/bin/ditto", &["-x", "-k", &archive.to_string_lossy(), &extracted.to_string_lossy()])?;
  let app = extracted.join(APP_NAME);
  if !app.is_dir() {
    return Err(UpdateError::new("The download does not contain AppForge.app."));
  }
  verify_app(&app, current_app, &release.version)?;
  let replacement = directory.join(APP_NAME);
  fs::rename(&app, &replacement).map_err(|e| UpdateError::new(format!("Could not stage the update: {e}")))?;
  Ok(replacement)
}

fn verify_app(app: &Path, current_app: &Path, version: &str) -> Result<(), UpdateError> {
  let path = app.to_string_lossy();
  run("/usr/bin/codesign", &["--verify", "--deep", "--strict", &path])
    .map_err(|e| UpdateError::new(format!("The update's code signature is invalid. {e}")))?;
  run("/usr/sbin/spctl", &["--assess", "--type", "execute", &path]).map_err(|e| UpdateError::new(format!("macOS rejected the update. {e}")))?;
  let current_team = signing_team(current_app)?;
  if current_team.is_none() || current_team != signing_team(app)? {
    return Err(UpdateError::new("The update must be signed by the same developer as the installed app."));
  }
  if bundle_value(app, "CFBundleIdentifier")?.trim() != BUNDLE_ID {
    return Err(UpdateError::new("The update is for a different application."));
  }
  if bundle_value(app, "CFBundleShortVersionString")?.trim() != version {
    return Err(UpdateError::new("The update's version does not match the release."));
  }
  Ok(())
}

fn bundle_value(app: &Path, key: &str) -> Result<String, UpdateError> {
  run("/usr/libexec/PlistBuddy", &["-c", &format!("Print :{key}"), &app.join("Contents/Info.plist").to_string_lossy()])
}

fn signing_team(app: &Path) -> Result<Option<String>, UpdateError> {
  run("/usr/bin/codesign", &["-dv", "--verbose=2", &app.to_string_lossy()]).map(|output| team_identifier(&output))
}

/// The `TeamIdentifier` from `codesign -dv` output; `None` for ad-hoc signatures.
pub fn team_identifier(output: &str) -> Option<String> {
  output
    .lines()
    .find_map(|line| line.strip_prefix("TeamIdentifier="))
    .map(str::trim)
    .filter(|team| *team != "not set" && !team.is_empty())
    .map(str::to_string)
}

/// Waits for this process to exit, swaps the staged app into place, and reopens
/// AppForge. The caller must quit right after this returns. A failed swap restores
/// the previous app and reopens it.
pub fn install(staged: &mut Staged, current_app: &Path) -> Result<(), UpdateError> {
  let script = staged.directory.path().join("swap.sh");
  fs::write(&script, SWAP_SCRIPT).map_err(|e| UpdateError::new(format!("Could not prepare the installer: {e}")))?;
  Command::new("/bin/bash")
    .arg(&script)
    .arg(std::process::id().to_string())
    .arg(current_app)
    .arg(&staged.app)
    .arg(staged.directory.path())
    .stdin(Stdio::null())
    .stdout(Stdio::null())
    .stderr(Stdio::null())
    // Its own process group, so it outlives this app.
    .process_group(0)
    .spawn()
    .map_err(|e| UpdateError::new(format!("Could not start the installer: {e}")))?;
  staged.directory.disable_cleanup(true);
  Ok(())
}

/// The installer script: waits for the app to quit, swaps the verified app in, and reopens it; a failed swap restores the old app.
pub const SWAP_SCRIPT: &str = r#"#!/bin/bash
pid="$1"; current="$2"; replacement="$3"; staging="$4"
for _ in $(seq 1 150); do kill -0 "$pid" 2>/dev/null || break; sleep 0.2; done
if kill -0 "$pid" 2>/dev/null; then exit 1; fi
backup="$staging/previous.app"
if mv "$current" "$backup" && mv "$replacement" "$current"; then
  if open "$current"; then
    rm -rf "$staging"
    exit 0
  fi
  mv "$current" "$replacement" || exit 1
fi
if [ -d "$backup" ] && [ ! -e "$current" ]; then
  mv "$backup" "$current" || exit 1
fi
open "$current"
"#;
