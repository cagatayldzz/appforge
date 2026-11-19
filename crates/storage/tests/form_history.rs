//! Local history keeps revision order, scope isolation and passwords outside Git.

use appforge_storage::{History, HistoryRevision, MemorySecrets};
use git2::{ObjectType, Repository};

#[test]
fn revisions_round_trip_without_storing_passwords_in_git() {
  let folder = tempfile::tempdir().unwrap();
  let secrets = MemorySecrets::default();
  let history = History::new(folder.path(), "account/app/review");
  let other = History::new(folder.path(), "account/other-app/review");
  assert!(history.list().unwrap().is_empty());

  let revision = HistoryRevision {
    before: [("notes".into(), "Old notes".into())].into(),
    after: [("notes".into(), "New notes".into())].into(),
    password_changed: true,
    password_before_present: true,
    password_after_present: true,
  };
  history.append(&revision, Some(("old-demo-password", "new-demo-password")), &secrets).unwrap();
  history
    .append(
      &HistoryRevision { after: [("notes".into(), "Newest notes".into())].into(), before: revision.after.clone(), ..HistoryRevision::default() },
      None,
      &secrets,
    )
    .unwrap();

  let entries = history.list().unwrap();
  assert_eq!(entries.len(), 2);
  assert_eq!(entries[0].2.after["notes"], "Newest notes");
  assert_eq!(entries[1].2.before["notes"], "Old notes");
  assert_eq!(History::password(&entries[1].0, true, &secrets).unwrap(), "old-demo-password");
  assert_eq!(History::password(&entries[1].0, false, &secrets).unwrap(), "new-demo-password");
  assert!(other.list().unwrap().is_empty());

  let repo = Repository::open_bare(folder.path().join("form-history.git")).unwrap();
  let commit = repo.find_commit(entries[1].0.parse().unwrap()).unwrap();
  let object = commit.tree().unwrap().get_path(std::path::Path::new("form.json")).unwrap().to_object(&repo).unwrap();
  assert_eq!(object.kind(), Some(ObjectType::Blob));
  let text = std::str::from_utf8(object.as_blob().unwrap().content()).unwrap();
  assert!(!text.contains("old-demo-password"));
  assert!(!text.contains("new-demo-password"));
}
