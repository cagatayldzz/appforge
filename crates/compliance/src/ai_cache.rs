//! The last completed AI review for one account, app, version and interface language.

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

pub struct AiCache(PathBuf);

impl AiCache {
  pub fn new(data_dir: &Path, issuer: &str, app: &str, version: Option<&str>, language: Language, project: bool) -> Self {
    let encode = |value: &str| value.as_bytes().iter().map(|byte| format!("{byte:02x}")).collect::<String>();
    let language = match language {
      Language::English => "en",
      Language::Turkish => "tr",
    };
    let mode = if project { "project" } else { "metadata" };
    Self(
      data_dir
        .join("ai-reviews")
        .join(encode(issuer))
        .join(encode(app))
        .join(format!("{}-{language}-{mode}.json", version.map_or_else(|| "no-version".to_owned(), encode))),
    )
  }

  pub fn load(&self) -> io::Result<Option<CachedReview>> {
    match fs::read(&self.0) {
      Ok(data) => serde_json::from_slice(&data).map(Some).map_err(io::Error::other),
      Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
      Err(error) => Err(error),
    }
  }

  pub fn save(&self, entry: &CachedReview) -> io::Result<()> {
    let parent = self.0.parent().ok_or_else(|| io::Error::other("Missing analysis cache folder"))?;
    fs::create_dir_all(parent)?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    serde_json::to_writer(&mut file, entry).map_err(io::Error::other)?;
    file.flush()?;
    file.persist(&self.0).map_err(|error| error.error)?;
    Ok(())
  }
}
