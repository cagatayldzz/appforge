//! Private local Git history for confirmed form saves, with passwords stored separately in Keychain.

use crate::{SecretKey, SecretStore};
use git2::{ErrorCode, ObjectType, Oid, Repository, RepositoryInitOptions, Signature};
use serde::{Deserialize, Serialize};
use std::{
  collections::BTreeMap,
  error::Error,
  fs,
  os::unix::fs::DirBuilderExt as _,
  path::{Path, PathBuf},
};

pub type HistoryValues = BTreeMap<String, String>;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct HistoryRevision {
  pub before: HistoryValues,
  pub after: HistoryValues,
  pub password_changed: bool,
  pub password_before_present: bool,
  pub password_after_present: bool,
}

#[derive(Clone)]
pub struct History {
  path: PathBuf,
  reference: String,
}

type HistoryResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

impl History {
  pub fn new(data_dir: &Path, scope: &str) -> Self {
    let id = Oid::hash_object(ObjectType::Blob, scope.as_bytes()).expect("hashing a history scope");
    Self { path: data_dir.join("form-history.git"), reference: format!("refs/heads/form-{id}") }
  }

  fn open(&self) -> HistoryResult<Repository> {
    if !self.path.exists() {
      match fs::DirBuilder::new().mode(0o700).recursive(true).create(&self.path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error.into()),
      }
    }
    Ok(match Repository::open_bare(&self.path) {
      Ok(repo) => repo,
      Err(error) if error.code() == ErrorCode::NotFound => {
        Repository::init_opts(&self.path, RepositoryInitOptions::new().bare(true).external_template(false))?
      }
      Err(error) => return Err(error.into()),
    })
  }

  pub fn append(&self, revision: &HistoryRevision, passwords: Option<(&str, &str)>, secrets: &dyn SecretStore) -> HistoryResult<()> {
    let repo = self.open()?;
    let mut transaction = repo.transaction()?;
    transaction.lock_ref(&self.reference)?;
    let parent = match repo.refname_to_id(&self.reference) {
      Ok(id) => Some(repo.find_commit(id)?),
      Err(error) if error.code() == ErrorCode::NotFound => None,
      Err(error) => return Err(error.into()),
    };
    let blob = repo.blob(&serde_json::to_vec(revision)?)?;
    let mut tree = repo.treebuilder(None)?;
    tree.insert("form.json", blob, 0o100644)?;
    let tree = repo.find_tree(tree.write()?)?;
    let signature = Signature::now("AppForge", "history@appforge.local")?;
    let parents: Vec<_> = parent.iter().collect();
    let message = format!("Save form changes for {}", self.reference);
    let id = repo.commit(None, &signature, &signature, &message, &tree, &parents)?;
    if let Some((before, after)) = passwords {
      if !before.is_empty() {
        secrets.set(SecretKey::HistoryPassword(id, true), before)?;
      }
      if !after.is_empty() {
        if let Err(error) = secrets.set(SecretKey::HistoryPassword(id, false), after) {
          let _ = secrets.delete(SecretKey::HistoryPassword(id, true));
          return Err(error.into());
        }
      }
    }
    transaction.set_target(&self.reference, id, Some(&signature), "Save form changes")?;
    transaction.commit()?;
    Ok(())
  }

  pub fn list(&self) -> HistoryResult<Vec<(String, i64, HistoryRevision)>> {
    if !self.path.exists() {
      return Ok(Vec::new());
    }
    let repo = Repository::open_bare(&self.path)?;
    let head = match repo.refname_to_id(&self.reference) {
      Ok(head) => head,
      Err(error) if error.code() == ErrorCode::NotFound => return Ok(Vec::new()),
      Err(error) => return Err(error.into()),
    };
    let mut walk = repo.revwalk()?;
    walk.push(head)?;
    walk
      .map(|id| {
        let commit = repo.find_commit(id?)?;
        let tree = commit.tree()?;
        let entry = tree.get_name("form.json").ok_or("History entry is missing its form data.")?;
        let revision = serde_json::from_slice(repo.find_blob(entry.id())?.content())?;
        Ok((commit.id().to_string(), commit.time().seconds(), revision))
      })
      .collect()
  }

  pub fn password(id: &str, before: bool, secrets: &dyn SecretStore) -> HistoryResult<String> {
    secrets
      .get(SecretKey::HistoryPassword(Oid::from_str(id)?, before))?
      .ok_or_else(|| "The password for this revision was not found in Keychain.".into())
  }
}
