use appforge_appstore::{App, AppInfoLocalization, AppSnapshot, ReviewDetail, ScreenshotSet, Version, VersionLocalization};
use appforge_compliance::{check, Category, Report, Severity, Verdict};
use appforge_storage::PrivacyDeclaration;

/// A snapshot that passes every rule.
fn ready() -> AppSnapshot {
  AppSnapshot {
    app: App { id: "1".into(), name: "Example".into(), bundle_id: "com.acme.example".into(), sku: "S".into(), primary_locale: "en-US".into(), ..Default::default() },
    general: Default::default(),
    info_state: "PREPARE_FOR_SUBMISSION".into(),
    info_localizations: vec![AppInfoLocalization {
      id: "i".into(),
      locale: "en-US".into(),
      name: "Example".into(),
      subtitle: "Plan your day".into(),
      privacy_policy_url: "https://acme.dev/privacy".into(),
      ..Default::default()
    }],
    version: Some(Version { id: "v".into(), version_string: "1.1".into(), state: "PREPARE_FOR_SUBMISSION".into(), supports_release_notes: true }),
    version_localizations: vec![VersionLocalization {
      id: "l".into(),
      locale: "en-US".into(),
      description: "Example helps you plan every day with reminders, lists and a calendar that works offline. Share lists with family and keep everything in sync across your devices.".into(),
      keywords: "planner,todo,calendar".into(),
      whats_new: "Faster lists and a new calendar view.".into(),
      support_url: "https://acme.dev/support".into(),
      ..Default::default()
    }],
    screenshot_sets: vec![ScreenshotSet { display_type: "APP_IPHONE_67".into(), count: 3 }],
    review_detail: Some(ReviewDetail {
      id: Some("r".into()),
      contact_first_name: "Ada".into(),
      contact_last_name: "Lovelace".into(),
      contact_phone: "+1 555 0100".into(),
      contact_email: "ada@acme.dev".into(),
      notes: "Open the app and tap Start.".into(),
      ..Default::default()
    }),
  }
}

fn declared() -> PrivacyDeclaration {
  PrivacyDeclaration { confirmed_in_app_store_connect: true, ..Default::default() }
}

fn ids(report: &Report) -> Vec<&str> {
  report.findings.iter().map(|finding| finding.id.as_str()).collect()
}

#[test]
fn a_complete_app_has_no_findings_and_looks_ready() {
  let report = check(&ready(), &declared());
  assert!(report.findings.is_empty(), "{:?}", ids(&report));
  assert_eq!(report.verdict(), Verdict::LooksReady);
  assert_eq!(report.score(), 100);
}

#[test]
fn limits_are_checked_in_every_locale() {
  let mut snapshot = ready();
  snapshot.info_localizations.push(AppInfoLocalization {
    id: "i2".into(),
    locale: "tr".into(),
    name: "x".repeat(31),
    subtitle: "y".repeat(31),
    ..Default::default()
  });
  snapshot.version_localizations[0].keywords = "k".repeat(101);
  snapshot.version_localizations[0].promotional_text = "p".repeat(171);
  let report = check(&snapshot, &declared());
  for id in ["info.name-too-long", "info.subtitle-too-long", "listing.keywords-too-long", "listing.promotional-too-long"] {
    assert!(ids(&report).contains(&id), "missing {id}: {:?}", ids(&report));
  }
  assert_eq!(report.verdict(), Verdict::NotReady);
}

#[test]
fn missing_support_and_privacy_urls_and_placeholders_are_critical() {
  let mut snapshot = ready();
  snapshot.version_localizations[0].support_url = "https://example.com/help".into();
  snapshot.info_localizations[0].privacy_policy_url = String::new();
  snapshot.version_localizations[0].description = "TODO write the description, lorem ipsum".into();
  let report = check(&snapshot, &declared());
  for id in ["listing.support-invalid", "privacy.policy-missing", "listing.placeholder"] {
    let finding = report.findings.iter().find(|finding| finding.id == id).unwrap_or_else(|| panic!("missing {id}"));
    assert_eq!(finding.severity, Severity::Critical);
  }
}

#[test]
fn empty_release_notes_are_critical_in_every_locale() {
  let mut snapshot = ready();
  snapshot.version_localizations[0].whats_new = "  ".into();
  let report = check(&snapshot, &declared());
  let finding = report.findings.iter().find(|finding| finding.id == "listing.whats-new-missing").expect("missing release notes finding");
  assert_eq!(finding.severity, Severity::Critical);
}

#[test]
fn first_release_does_not_require_release_notes() {
  let mut snapshot = ready();
  snapshot.version.as_mut().unwrap().supports_release_notes = false;
  snapshot.version_localizations[0].whats_new.clear();
  assert!(!ids(&check(&snapshot, &declared())).contains(&"listing.whats-new-missing"));
}

#[test]
fn words_that_merely_contain_a_placeholder_are_not_flagged() {
  let mut snapshot = ready();
  snapshot.version_localizations[0].description =
    format!("{} Mastodon and tbdsomething are fine; a todolist app.", snapshot.version_localizations[0].description);
  assert!(!ids(&check(&snapshot, &declared())).contains(&"listing.placeholder"));
}

#[test]
fn other_platforms_in_the_listing_are_a_warning() {
  let mut snapshot = ready();
  snapshot.version_localizations[0].whats_new = "Now also on Android".into();
  let report = check(&snapshot, &declared());
  assert!(report.findings.iter().any(|finding| finding.id == "listing.other-platform" && finding.severity == Severity::Warning));
  assert_eq!(report.verdict(), Verdict::NeedsAttention);
}

#[test]
fn no_screenshots_and_no_review_detail_are_critical() {
  let mut snapshot = ready();
  snapshot.screenshot_sets.clear();
  snapshot.review_detail = None;
  let report = check(&snapshot, &declared());
  assert!(ids(&report).contains(&"listing.screenshots-missing"));
  let review = report.findings.iter().find(|finding| finding.id == "review.missing").unwrap();
  assert_eq!(review.category, Category::AppReview);
}

#[test]
fn a_demo_account_is_required_when_the_app_needs_a_sign_in() {
  let mut snapshot = ready();
  snapshot.review_detail.as_mut().unwrap().demo_account_required = true;
  assert!(ids(&check(&snapshot, &declared())).contains(&"review.demo-account-missing"));
  let review = snapshot.review_detail.as_mut().unwrap();
  review.demo_account_name = "demo".into();
  review.demo_account_password = "secret".into();
  assert!(!ids(&check(&snapshot, &declared())).contains(&"review.demo-account-missing"));
}

#[test]
fn a_login_wall_declared_locally_but_not_marked_is_flagged() {
  let privacy = PrivacyDeclaration { requires_login_to_review: true, ..declared() };
  let report = check(&ready(), &privacy);
  assert!(ids(&report).contains(&"review.demo-account-missing"), "no demo account was entered: {:?}", ids(&report));
}

#[test]
fn an_incomplete_contact_names_what_is_missing() {
  let mut snapshot = ready();
  snapshot.review_detail.as_mut().unwrap().contact_phone = String::new();
  let report = check(&snapshot, &declared());
  let finding = report.findings.iter().find(|finding| finding.id == "review.contact-incomplete").unwrap();
  assert!(finding.detail.contains("phone"));
}

#[test]
fn privacy_answers_drive_the_privacy_findings() {
  let privacy = PrivacyDeclaration { uses_tracking: true, has_accounts: true, third_party_sdks: "Firebase".into(), ..Default::default() };
  let report = check(&ready(), &privacy);
  for id in ["privacy.not-confirmed", "privacy.tracking-no-prompt", "privacy.account-deletion"] {
    assert!(ids(&report).contains(&id), "missing {id}");
  }
  let sdks_only = PrivacyDeclaration { confirmed_in_app_store_connect: true, third_party_sdks: "Firebase".into(), ..Default::default() };
  assert!(ids(&check(&ready(), &sdks_only)).contains(&"privacy.sdk-mismatch"));
}

#[test]
fn findings_are_sorted_with_the_worst_first_and_the_score_drops() {
  let mut snapshot = ready();
  snapshot.version_localizations[0].keywords = String::new();
  snapshot.screenshot_sets.clear();
  let report = check(&snapshot, &PrivacyDeclaration::default());
  let severities: Vec<Severity> = report.findings.iter().map(|finding| finding.severity).collect();
  let mut sorted = severities.clone();
  sorted.sort();
  assert_eq!(severities, sorted);
  assert_eq!(
    report.score(),
    100 - 20 * report.count(Severity::Critical) as u32 - 7 * report.count(Severity::Warning) as u32 - report.count(Severity::Info) as u32
  );
}

#[test]
fn an_app_without_an_editable_version_is_reported_instead_of_crashing() {
  let mut snapshot = ready();
  snapshot.version = None;
  snapshot.version_localizations.clear();
  snapshot.review_detail = None;
  assert!(ids(&check(&snapshot, &declared())).contains(&"listing.no-version"));
}
