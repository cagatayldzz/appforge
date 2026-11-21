//! Saved reviews round-trip, keep their history and stay scoped to the analyzed app version.

use appforge_ai::Provider;
use appforge_compliance::{AiCache, AiReview, CachedReview};
use appforge_storage::Language;

fn review(finished_at: u64, summary: &str, project_included: bool) -> CachedReview {
  CachedReview {
    finished_at,
    provider: Provider::OpenAi,
    model: "test-model".into(),
    project_included,
    review: AiReview { summary: summary.into(), findings: vec![] },
  }
}

#[test]
fn every_review_is_kept_and_the_newest_is_the_latest() {
  let temp = tempfile::tempdir().unwrap();
  let cache = AiCache::new(temp.path(), "account", "app", Some("version-1"), Language::Turkish);
  let other_version = AiCache::new(temp.path(), "account", "app", Some("version-2"), Language::Turkish);
  assert!(cache.latest().unwrap().is_none());
  cache.save(&review(42, "First", true)).unwrap();
  cache.save(&review(50, "Second", false)).unwrap();
  cache.save(&review(45, "Between", true)).unwrap();
  let history = cache.history().unwrap();
  assert_eq!(history.iter().map(|entry| entry.finished_at).collect::<Vec<_>>(), [50, 45, 42]);
  assert_eq!(cache.latest().unwrap().unwrap().review.summary, "Second");
  assert!(other_version.latest().unwrap().is_none());
}

#[test]
fn a_review_saved_by_an_earlier_build_still_shows_up() {
  let temp = tempfile::tempdir().unwrap();
  let cache = AiCache::new(temp.path(), "account", "app", None, Language::English);
  let app_dir = temp.path().join("ai-reviews").join("6163636f756e74").join("617070");
  std::fs::create_dir_all(&app_dir).unwrap();
  std::fs::write(app_dir.join("no-version-en-project.json"), serde_json::to_vec(&review(10, "Old", true)).unwrap()).unwrap();
  cache.save(&review(20, "New", true)).unwrap();
  let history = cache.history().unwrap();
  assert_eq!(history.iter().map(|entry| entry.review.summary.as_str()).collect::<Vec<_>>(), ["New", "Old"]);
}
