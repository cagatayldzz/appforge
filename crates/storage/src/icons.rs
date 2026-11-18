//! The icons of the user's apps, kept as files in the data folder so they show at once on the next launch.

use std::{
  fs, io,
  path::{Path, PathBuf},
  time::{Duration, SystemTime},
};

/// An icon younger than this is not asked for again.
const MAX_AGE: Duration = Duration::from_secs(24 * 60 * 60);

/// The folder of the icons: one file per app, named by its App Store Connect id.
#[derive(Clone, Debug)]
pub struct IconCache {
  dir: PathBuf,
}

impl IconCache {
  pub fn new(data_dir: &Path) -> Self {
    Self { dir: data_dir.join("icons") }
  }

  /// `None` when the id is not safe to use as a file name.
  fn file(&self, app_id: &str) -> Option<PathBuf> {
    let safe = !app_id.is_empty() && app_id.chars().all(|character| character.is_ascii_alphanumeric() || character == '-' || character == '_');
    safe.then(|| self.dir.join(format!("{app_id}.png")))
  }

  /// The stored icon of an app, if there is one.
  pub fn path(&self, app_id: &str) -> Option<PathBuf> {
    self.file(app_id).filter(|file| file.is_file())
  }

  /// Whether the stored icon is recent enough not to ask for it again.
  pub fn is_fresh(&self, app_id: &str) -> bool {
    let Some(file) = self.path(app_id) else { return false };
    let age = fs::metadata(file).and_then(|metadata| metadata.modified()).ok().and_then(|modified| SystemTime::now().duration_since(modified).ok());
    age.is_some_and(|age| age < MAX_AGE)
  }

  /// Stores an icon. Only PNG and JPEG files are kept: anything else is not an icon.
  pub fn store(&self, app_id: &str, bytes: &[u8]) -> io::Result<PathBuf> {
    let invalid = |message: &str| io::Error::new(io::ErrorKind::InvalidData, message.to_owned());
    let file = self.file(app_id).ok_or_else(|| invalid("the app id is not usable as a file name"))?;
    if !(bytes.starts_with(b"\x89PNG\r\n\x1a\n") || bytes.starts_with(&[0xFF, 0xD8, 0xFF])) {
      return Err(invalid("the file is not a PNG or JPEG image"));
    }
    fs::create_dir_all(&self.dir)?;
    // Written next to its place and renamed, so a half-written file is never shown.
    let partial = file.with_extension("png.partial");
    fs::write(&partial, bytes)?;
    fs::rename(&partial, &file)?;
    Ok(file)
  }
}
