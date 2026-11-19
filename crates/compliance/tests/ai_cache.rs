//! Saved reviews round-trip and stay scoped to the analyzed app version.

use appforge_ai::Provider;
use appforge_compliance::{AiCache, AiReview, CachedReview};
use appforge_storage::Language;

#[test]
fn the_last_successful_review_can_be_reused_without_an_ai_request() {
  let temp = tempfile::tempdir().unwrap();
  let cache = AiCache::new(temp.path(), "account", "app", Some("version-1"), Language::Turkish, true);
  let other_version = AiCache::new(temp.path(), "account", "app", Some("version-2"), Language::Turkish, true);
  let metadata_only = AiCache::new(temp.path(), "account", "app", Some("version-1"), Language::Turkish, false);
  assert!(cache.load().unwrap().is_none());
  let review = CachedReview {
    finished_at: 42,
    provider: Provider::OpenAi,
    model: "test-model".into(),
    project_included: true,
    review: AiReview { summary: "Saved review".into(), findings: vec![] },
  };
  cache.save(&review).unwrap();
  let loaded = cache.load().unwrap().unwrap();
  assert_eq!(loaded.finished_at, 42);
  assert_eq!(loaded.review, review.review);
  assert!(other_version.load().unwrap().is_none());
  assert!(metadata_only.load().unwrap().is_none());
}
