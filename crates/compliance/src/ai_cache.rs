//! Every completed AI review for one account, app, version and interface language, newest first.

use crate::AiReview;
use appforge_ai::Provider;
use appforge_storage::Language;
use serde::{Deserialize, Serialize};
use std::{
  fs,
  io::{self, Write as _},
  path::{Path, PathBuf},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CachedReview {
  pub finished_at: u64,
  pub provider: Provider,
  pub model: String,
  pub project_included: bool,
  pub review: AiReview,
}

/// The folder of one scope holds one file per completed review, named by its finish time, so no analysis is overwritten.
pub struct AiCache {
  dir: PathBuf,
  /// Where earlier builds kept the single latest review of each mode.
  legacy: [PathBuf; 2],
}

impl AiCache {
  pub fn new(data_dir: &Path, issuer: &str, app: &str, version: Option<&str>, language: Language) -> Self {
    let encode = |value: &str| value.as_bytes().iter().map(|byte| format!("{byte:02x}")).collect::<String>();
    let language = match language {
      Language::English => "en",
      Language::Turkish => "tr",
    };
    let app_dir = data_dir.join("ai-reviews").join(encode(issuer)).join(encode(app));
    let version = version.map_or_else(|| "no-version".to_owned(), encode);
    Self {
      dir: app_dir.join(format!("{version}-{language}")),
      legacy: ["project", "metadata"].map(|mode| app_dir.join(format!("{version}-{language}-{mode}.json"))),
    }
  }

  /// All saved reviews, newest first. A file that cannot be read is skipped so one bad file does not hide the rest.
  pub fn history(&self) -> io::Result<Vec<CachedReview>> {
    let mut paths = self.legacy.to_vec();
    match fs::read_dir(&self.dir) {
      Ok(entries) => paths.extend(entries.filter_map(|entry| entry.ok()).map(|entry| entry.path())),
      Err(error) if error.kind() == io::ErrorKind::NotFound => {}
      Err(error) => return Err(error),
    }
    let mut reviews: Vec<CachedReview> = paths
      .iter()
      .filter(|path| path.extension().is_some_and(|extension| extension == "json"))
      .filter_map(|path| fs::read(path).ok())
      .filter_map(|data| serde_json::from_slice(&data).ok())
      .collect();
    reviews.sort_by_key(|entry| std::cmp::Reverse(entry.finished_at));
    Ok(reviews)
  }

  /// The most recent review, whatever it included.
  pub fn latest(&self) -> io::Result<Option<CachedReview>> {
    Ok(self.history()?.into_iter().next())
  }

  /// Adds a review to the history.
  pub fn save(&self, entry: &CachedReview) -> io::Result<()> {
    fs::create_dir_all(&self.dir)?;
    let mode = if entry.project_included { "project" } else { "metadata" };
    let path = self.dir.join(format!("{:020}-{mode}.json", entry.finished_at));
    let mut file = tempfile::NamedTempFile::new_in(&self.dir)?;
    serde_json::to_writer(&mut file, entry).map_err(io::Error::other)?;
    file.flush()?;
    file.persist(path).map_err(|error| error.error)?;
    Ok(())
  }
}
