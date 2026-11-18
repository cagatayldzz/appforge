use std::{
  fs,
  path::{Path, PathBuf},
  time::{Duration, SystemTime, UNIX_EPOCH},
};

pub const PLAYBOOK_REPOSITORY: &str = "https://github.com/mjmirza/app-store-compliance";
const RAW_BASE_URL: &str = "https://raw.githubusercontent.com/mjmirza/app-store-compliance/main/docs";
/// The playbook pages that carry the Apple rules: the rejection map, the checklist and the root causes.
const DOCUMENTS: [&str; 3] = ["APPLE.md", "PRE-SUBMISSION-CHECKLIST.md", "MISTAKE-PATTERNS.md"];
const CACHE_FOLDER: &str = "playbook";
const FETCHED_AT_FILE: &str = "fetched-at";

/// The rules of the community playbook, downloaded from GitHub and cached under the data directory.
/// They are third-party text: they are given to the AI as reference material, never as instructions.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Playbook {
  /// `(file name, markdown)`
  pub documents: Vec<(String, String)>,
  /// Seconds since the Unix epoch.
  pub fetched_at: Option<u64>,
}

fn cache_dir(data_dir: &Path) -> PathBuf {
  data_dir.join(CACHE_FOLDER)
}

impl Playbook {
  pub fn is_empty(&self) -> bool {
    self.documents.is_empty()
  }

  /// Reads the cached copy; empty when there is none.
  pub fn load(data_dir: &Path) -> Self {
    let dir = cache_dir(data_dir);
    let documents: Vec<(String, String)> =
      DOCUMENTS.iter().filter_map(|name| fs::read_to_string(dir.join(name)).ok().map(|text| ((*name).to_owned(), text))).collect();
    let fetched_at = fs::read_to_string(dir.join(FETCHED_AT_FILE)).ok().and_then(|text| text.trim().parse().ok());
    if documents.len() == DOCUMENTS.len() {
      Self { documents, fetched_at }
    } else {
      Self::default()
    }
  }

  /// Downloads the playbook from GitHub and replaces the cache. Blocking.
  pub fn refresh(data_dir: &Path) -> Result<Self, String> {
    Self::refresh_from(data_dir, RAW_BASE_URL)
  }

  /// Like [`Playbook::refresh`] from another base URL; used by tests. The cache is only replaced when every page arrived.
  pub fn refresh_from(data_dir: &Path, base_url: &str) -> Result<Self, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(30))).build().into();
    let mut documents = Vec::new();
    for name in DOCUMENTS {
      let url = format!("{}/{name}", base_url.trim_end_matches('/'));
      let mut response = agent.get(&url).call().map_err(|error| format!("Could not download {name}: {error}"))?;
      let text = response.body_mut().read_to_string().map_err(|error| format!("Could not read {name}: {error}"))?;
      if text.trim().is_empty() {
        return Err(format!("{name} was empty"));
      }
      documents.push((name.to_owned(), text));
    }
    let fetched_at = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let dir = cache_dir(data_dir);
    fs::create_dir_all(&dir).map_err(|error| format!("Could not save the playbook: {error}"))?;
    for (name, text) in &documents {
      fs::write(dir.join(name), text).map_err(|error| format!("Could not save {name}: {error}"))?;
    }
    fs::write(dir.join(FETCHED_AT_FILE), fetched_at.to_string()).map_err(|error| format!("Could not save the playbook: {error}"))?;
    Ok(Self { documents, fetched_at: Some(fetched_at) })
  }

  /// The pages joined into one text of at most `limit` characters, each page announced by its name.
  pub fn text(&self, limit: usize) -> String {
    let mut text = String::new();
    for (name, markdown) in &self.documents {
      let remaining = limit.saturating_sub(text.chars().count());
      if remaining == 0 {
        break;
      }
      text.push_str(&format!("### {name}\n"));
      text.extend(markdown.chars().take(remaining));
      text.push_str("\n\n");
    }
    text
  }
}
