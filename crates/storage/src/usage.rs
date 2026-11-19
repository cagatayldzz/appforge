//! What the AI features have used: requests, tokens and an estimated cost per model, kept in `ai-usage.json`.

use appforge_ai::{Provider, Usage};
use serde::{Deserialize, Serialize};
use std::{
  fs, io,
  path::{Path, PathBuf},
};

/// The file name in the data directory.
pub const USAGE_FILE: &str = "ai-usage.json";

/// What one model has used.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct UsageTotals {
  pub requests: u64,
  pub input_tokens: u64,
  pub output_tokens: u64,
  /// US dollars, from the list price of the model when each request was made.
  pub cost: f64,
}

impl UsageTotals {
  pub fn tokens(&self) -> u64 {
    self.input_tokens + self.output_tokens
  }

  fn add(&mut self, other: &UsageTotals) {
    self.requests += other.requests;
    self.input_tokens += other.input_tokens;
    self.output_tokens += other.output_tokens;
    self.cost += other.cost;
  }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ModelUsage {
  pub provider: Provider,
  pub model: String,
  #[serde(flatten)]
  pub totals: UsageTotals,
}

/// The running totals of AI use, since `since` (Unix seconds; 0 when nothing was counted yet).
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct UsageLedger {
  pub since: u64,
  pub models: Vec<ModelUsage>,
}

impl UsageLedger {
  /// A missing file is an empty ledger; an unreadable one is an error so it is never overwritten silently.
  pub fn load(dir: &Path) -> io::Result<Self> {
    match fs::read(Self::path(dir)) {
      Ok(data) => serde_json::from_slice(&data).map_err(io::Error::other),
      Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
      Err(error) => Err(error),
    }
  }

  pub fn path(dir: &Path) -> PathBuf {
    dir.join(USAGE_FILE)
  }

  pub fn save(&self, dir: &Path) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    let path = Self::path(dir);
    let data = serde_json::to_vec_pretty(self).map_err(io::Error::other)?;
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, data)?;
    fs::rename(temporary, path)
  }

  /// Adds one request. `now` is Unix seconds; it becomes `since` when this is the first request counted.
  pub fn record(&mut self, provider: Provider, model: &str, usage: Usage, now: u64) {
    if self.since == 0 {
      self.since = now;
    }
    let cost = provider.model_info(model).map_or(0., |info| info.cost(usage));
    let index = match self.models.iter().position(|entry| entry.provider == provider && entry.model == model) {
      Some(index) => index,
      None => {
        self.models.push(ModelUsage { provider, model: model.to_owned(), totals: UsageTotals::default() });
        self.models.len() - 1
      }
    };
    let totals = &mut self.models[index].totals;
    totals.requests += 1;
    totals.input_tokens += usage.input_tokens;
    totals.output_tokens += usage.output_tokens;
    totals.cost += cost;
  }

  /// Everything, over all models.
  pub fn total(&self) -> UsageTotals {
    let mut total = UsageTotals::default();
    for entry in &self.models {
      total.add(&entry.totals);
    }
    total
  }

  /// Starts counting from zero; the start date is set again by the next request.
  pub fn reset(&mut self) {
    *self = Self::default();
  }
}
