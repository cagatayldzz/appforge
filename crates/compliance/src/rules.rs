//! The built-in rules that check an app snapshot.

use crate::{Category, Finding, Report, Severity};
use appforge_appstore::{AppInfoLocalization, AppSnapshot, ReviewDetail, VersionLocalization};
use appforge_storage::PrivacyDeclaration;

const NAME_LIMIT: usize = 30;
const SUBTITLE_LIMIT: usize = 30;
const KEYWORDS_LIMIT: usize = 100;
const PROMOTIONAL_LIMIT: usize = 170;
const DESCRIPTION_LIMIT: usize = 4000;
const THIN_DESCRIPTION: usize = 150;
const PLACEHOLDERS: [&str; 6] = ["lorem ipsum", "todo", "tbd", "placeholder", "coming soon", "your text here"];
const OTHER_PLATFORMS: [&str; 5] = ["android", "google play", "play store", "windows phone", "blackberry"];

struct Rules(Vec<Finding>);

impl Rules {
  fn add(&mut self, id: &str, severity: Severity, category: Category, guideline: &str, title: &str, detail: &str, fix: &str) {
    self.0.push(Finding {
      id: id.to_owned(),
      severity,
      category,
      title: title.to_owned(),
      guideline: guideline.to_owned(),
      detail: detail.to_owned(),
      fix: fix.to_owned(),
    });
  }
}

fn length(text: &str) -> usize {
  text.chars().count()
}

fn is_url(text: &str) -> bool {
  text.starts_with("https://") || text.starts_with("http://")
}

fn is_placeholder_url(text: &str) -> bool {
  let lower = text.to_lowercase();
  ["example.com", "example.org", "localhost", "127.0.0.1", "yourdomain", "your-domain"].iter().any(|marker| lower.contains(marker))
}

fn contains_placeholder(text: &str) -> Option<&'static str> {
  let lower = text.to_lowercase();
  PLACEHOLDERS.iter().copied().find(|marker| {
    // "todo" and "tbd" only count as whole words.
    lower.split(|character: char| !character.is_alphanumeric()).any(|word| word == *marker) || (marker.contains(' ') && lower.contains(marker))
  })
}

/// Runs every built-in rule. The rules only know what App Store Connect's API and the user's privacy answers say;
/// they cannot see the binary, so a clean report is not a promise of approval.
pub fn check(snapshot: &AppSnapshot, privacy: &PrivacyDeclaration) -> Report {
  let mut rules = Rules(Vec::new());
  app_information(&mut rules, snapshot);
  listing(&mut rules, snapshot);
  review(&mut rules, snapshot, privacy);
  app_privacy(&mut rules, snapshot, privacy);
  rules.0.sort_by_key(|finding| finding.severity);
  Report { findings: rules.0 }
}

fn primary<'a, T>(items: &'a [T], locale_of: impl Fn(&T) -> &str, primary_locale: &str) -> Option<&'a T> {
  items.iter().find(|item| locale_of(item) == primary_locale).or_else(|| items.first())
}

fn app_information(rules: &mut Rules, snapshot: &AppSnapshot) {
  let Some(info) = primary(&snapshot.info_localizations, |item: &AppInfoLocalization| &item.locale, &snapshot.app.primary_locale) else {
    rules.add(
      "info.missing",
      Severity::Critical,
      Category::AppInformation,
      "2.3",
      "No App Information found",
      "App Store Connect returned no name or subtitle for this app.",
      "Open App Information and add a name for the primary language.",
    );
    return;
  };
  for localization in &snapshot.info_localizations {
    let place = format!(" ({})", localization.locale);
    if localization.name.trim().is_empty() {
      rules.add(
        "info.name-missing",
        Severity::Critical,
        Category::AppInformation,
        "2.3.7",
        &format!("App name is empty{place}"),
        "Every language needs an app name.",
        "Enter the app name.",
      );
    } else if length(&localization.name) > NAME_LIMIT {
      rules.add(
        "info.name-too-long",
        Severity::Critical,
        Category::AppInformation,
        "2.3.7",
        &format!("App name is longer than {NAME_LIMIT} characters{place}"),
        &format!("The name has {} characters.", length(&localization.name)),
        "Shorten the name; put descriptive words in the subtitle and keywords.",
      );
    }
    if length(&localization.subtitle) > SUBTITLE_LIMIT {
      rules.add(
        "info.subtitle-too-long",
        Severity::Critical,
        Category::AppInformation,
        "2.3.7",
        &format!("Subtitle is longer than {SUBTITLE_LIMIT} characters{place}"),
        &format!("The subtitle has {} characters.", length(&localization.subtitle)),
        "Shorten the subtitle.",
      );
    }
  }
  if info.subtitle.trim().is_empty() {
    rules.add(
      "info.subtitle-missing",
      Severity::Info,
      Category::AppInformation,
      "2.3.7",
      "No subtitle",
      "A subtitle helps people understand the app in search results.",
      "Add a short subtitle that says what the app does.",
    );
  }
  for (field, value) in [("name", &info.name), ("subtitle", &info.subtitle)] {
    if let Some(marker) = contains_placeholder(value) {
      rules.add(
        "info.placeholder",
        Severity::Critical,
        Category::AppInformation,
        "2.3.1",
        &format!("The {field} looks like placeholder text (\"{marker}\")"),
        "Apple rejects apps with placeholder or unfinished metadata.",
        "Replace it with the real text.",
      );
    }
  }
}

fn listing(rules: &mut Rules, snapshot: &AppSnapshot) {
  let Some(version) = &snapshot.version else {
    rules.add(
      "listing.no-version",
      Severity::Warning,
      Category::AppInformation,
      "",
      "No iOS version in preparation",
      "There is no app version that can still be edited, so the listing and review details cannot be checked.",
      "Create a new version in App Store Connect.",
    );
    return;
  };
  if matches!(version.state.as_str(), "REJECTED" | "METADATA_REJECTED" | "INVALID_BINARY") {
    rules.add(
      "listing.rejected",
      Severity::Info,
      Category::AppReview,
      "",
      &format!("Version {} was rejected", version.version_string),
      "Read Apple's message in the Resolution Center before you change anything.",
      "Fix exactly what Apple names, answer in the Resolution Center and resubmit.",
    );
  }
  let Some(listing) = primary(&snapshot.version_localizations, |item: &VersionLocalization| &item.locale, &snapshot.app.primary_locale) else {
    rules.add(
      "listing.missing",
      Severity::Critical,
      Category::AppInformation,
      "2.3",
      "No store listing text",
      "The version has no localization with a description.",
      "Add a description, keywords and a support URL.",
    );
    return;
  };
  for localization in &snapshot.version_localizations {
    let place = format!(" ({})", localization.locale);
    if localization.description.trim().is_empty() {
      rules.add(
        "listing.description-missing",
        Severity::Critical,
        Category::AppInformation,
        "2.3",
        &format!("Description is empty{place}"),
        "A description is required.",
        "Describe what the app does and who it is for.",
      );
    } else if length(&localization.description) > DESCRIPTION_LIMIT {
      rules.add(
        "listing.description-too-long",
        Severity::Critical,
        Category::AppInformation,
        "2.3",
        &format!("Description is longer than {DESCRIPTION_LIMIT} characters{place}"),
        &format!("The description has {} characters.", length(&localization.description)),
        "Shorten it.",
      );
    }
    if length(&localization.keywords) > KEYWORDS_LIMIT {
      rules.add(
        "listing.keywords-too-long",
        Severity::Critical,
        Category::AppInformation,
        "2.3.7",
        &format!("Keywords are longer than {KEYWORDS_LIMIT} characters{place}"),
        &format!("The keywords have {} characters.", length(&localization.keywords)),
        "Remove keywords; separate them with commas and no spaces.",
      );
    }
    if length(&localization.promotional_text) > PROMOTIONAL_LIMIT {
      rules.add(
        "listing.promotional-too-long",
        Severity::Critical,
        Category::AppInformation,
        "2.3",
        &format!("Promotional text is longer than {PROMOTIONAL_LIMIT} characters{place}"),
        &format!("The promotional text has {} characters.", length(&localization.promotional_text)),
        "Shorten it.",
      );
    }
  }
  if listing.description.trim().chars().count() > 0 && length(listing.description.trim()) < THIN_DESCRIPTION {
    rules.add(
      "listing.description-thin",
      Severity::Warning,
      Category::AppInformation,
      "4.2",
      "The description is very short",
      "Thin listings often go with apps Apple considers minimal (guideline 4.2).",
      "Explain the main features and what makes the app useful.",
    );
  }
  if listing.keywords.trim().is_empty() {
    rules.add(
      "listing.keywords-missing",
      Severity::Warning,
      Category::AppInformation,
      "2.3.7",
      "No keywords",
      "Keywords drive App Store search.",
      "Add up to 100 characters of comma separated keywords.",
    );
  }
  let text = format!("{} {} {}", listing.description, listing.promotional_text, listing.whats_new).to_lowercase();
  if let Some(platform) = OTHER_PLATFORMS.iter().find(|platform| text.contains(*platform)) {
    rules.add(
      "listing.other-platform",
      Severity::Warning,
      Category::AppInformation,
      "2.3.10",
      &format!("The listing mentions another platform (\"{platform}\")"),
      "Apple does not allow metadata that mentions other mobile platforms.",
      "Remove the reference.",
    );
  }
  for (field, value) in [("description", &listing.description), ("promotional text", &listing.promotional_text), ("What's New", &listing.whats_new)] {
    if let Some(marker) = contains_placeholder(value) {
      rules.add(
        "listing.placeholder",
        Severity::Critical,
        Category::AppInformation,
        "2.3.1",
        &format!("The {field} looks like placeholder text (\"{marker}\")"),
        "Apple rejects apps with placeholder or unfinished metadata.",
        "Replace it with the real text.",
      );
    }
  }
  if listing.support_url.trim().is_empty() {
    rules.add(
      "listing.support-missing",
      Severity::Critical,
      Category::AppInformation,
      "1.5",
      "No support URL",
      "Apple requires a support URL with real contact information.",
      "Add a page where people can reach you.",
    );
  } else if !is_url(&listing.support_url) || is_placeholder_url(&listing.support_url) {
    rules.add(
      "listing.support-invalid",
      Severity::Critical,
      Category::AppInformation,
      "1.5",
      "The support URL is not a real web address",
      &format!("\"{}\" is not usable.", listing.support_url),
      "Use a working https:// page.",
    );
  } else if !listing.support_url.starts_with("https://") {
    rules.add(
      "listing.support-http",
      Severity::Warning,
      Category::AppInformation,
      "1.5",
      "The support URL is not https",
      "Reviewers and users expect a secure page.",
      "Use an https:// address.",
    );
  }
  if snapshot.screenshot_sets.iter().all(|set| set.count == 0) {
    rules.add(
      "listing.screenshots-missing",
      Severity::Critical,
      Category::AppInformation,
      "2.3.3",
      "No screenshots",
      "The primary language has no screenshots, and App Store Connect does not accept a submission without them.",
      "Upload screenshots for the required device sizes.",
    );
  }
}

fn review(rules: &mut Rules, snapshot: &AppSnapshot, privacy: &PrivacyDeclaration) {
  let Some(version) = &snapshot.version else { return };
  let Some(review) = &snapshot.review_detail else {
    rules.add(
      "review.missing",
      Severity::Critical,
      Category::AppReview,
      "2.1",
      "No App Review information",
      &format!("Version {} has no contact details, demo account or notes for the reviewer.", version.version_string),
      "Open App Review and add a contact; add a demo account and notes if the app needs them.",
    );
    return;
  };
  contact(rules, review);
  let needs_account = review.demo_account_required || privacy.requires_login_to_review;
  if needs_account && (review.demo_account_name.trim().is_empty() || review.demo_account_password.trim().is_empty()) {
    rules.add(
      "review.demo-account-missing",
      Severity::Critical,
      Category::AppReview,
      "2.1",
      "The reviewer cannot sign in",
      "The app needs a sign-in, but the demo account name or password is empty. A reviewer who cannot reach a feature rejects it.",
      "Add a working demo account (and keep its backend running during review) in App Review.",
    );
  } else if privacy.requires_login_to_review && !review.demo_account_required {
    rules.add(
      "review.demo-account-not-marked",
      Severity::Warning,
      Category::AppReview,
      "2.1",
      "Sign-in required, but \"Sign-in required\" is off",
      "You said the reviewer must sign in, but App Review does not mark a demo account as required.",
      "Turn on \"Sign-in required\" and enter the demo account.",
    );
  }
  if review.notes.trim().is_empty() {
    rules.add(
      "review.notes-empty",
      Severity::Info,
      Category::AppReview,
      "2.1",
      "No notes for the reviewer",
      "Notes are the place to explain anything that is not obvious: hardware needed, hidden features, in-app purchases, regional behaviour.",
      "Write a short walk-through of the main flows and of anything the reviewer cannot discover alone.",
    );
  }
}

fn contact(rules: &mut Rules, review: &ReviewDetail) {
  let missing: Vec<&str> = [
    ("first name", &review.contact_first_name),
    ("last name", &review.contact_last_name),
    ("phone", &review.contact_phone),
    ("email", &review.contact_email),
  ]
  .into_iter()
  .filter(|(_, value)| value.trim().is_empty())
  .map(|(label, _)| label)
  .collect();
  if !missing.is_empty() {
    rules.add(
      "review.contact-incomplete",
      Severity::Critical,
      Category::AppReview,
      "2.1",
      "The review contact is incomplete",
      &format!("Missing: {}.", missing.join(", ")),
      "Fill in the contact so Apple can reach you during review.",
    );
  } else if !review.contact_email.contains('@') {
    rules.add(
      "review.contact-email",
      Severity::Warning,
      Category::AppReview,
      "2.1",
      "The review contact email looks invalid",
      &format!("\"{}\" has no @.", review.contact_email),
      "Enter a valid email address.",
    );
  }
}

fn app_privacy(rules: &mut Rules, snapshot: &AppSnapshot, privacy: &PrivacyDeclaration) {
  let policy = primary(&snapshot.info_localizations, |item: &AppInfoLocalization| &item.locale, &snapshot.app.primary_locale)
    .map(|info| info.privacy_policy_url.trim().to_owned());
  match policy.as_deref() {
    Some("") | None => rules.add(
      "privacy.policy-missing",
      Severity::Critical,
      Category::AppPrivacy,
      "5.1.1",
      "No privacy policy URL",
      "Every app needs a privacy policy link in App Store Connect.",
      "Publish a privacy policy page and add its address in App Privacy.",
    ),
    Some(url) if !is_url(url) || is_placeholder_url(url) => rules.add(
      "privacy.policy-invalid",
      Severity::Critical,
      Category::AppPrivacy,
      "5.1.1",
      "The privacy policy URL is not a real web address",
      &format!("\"{url}\" is not usable."),
      "Use a working https:// page.",
    ),
    Some(url) if !url.starts_with("https://") => rules.add(
      "privacy.policy-http",
      Severity::Warning,
      Category::AppPrivacy,
      "5.1.1",
      "The privacy policy URL is not https",
      "Users and reviewers expect a secure page.",
      "Use an https:// address.",
    ),
    _ => {}
  }
  if !privacy.confirmed_in_app_store_connect {
    rules.add(
      "privacy.not-confirmed",
      Severity::Warning,
      Category::AppPrivacy,
      "5.1.1",
      "App Privacy answers are not confirmed",
      "Apple's API does not expose the App Privacy (nutrition label) answers, so AppForge cannot read them.",
      "Check them in App Store Connect and tick \"I answered App Privacy\" in App Privacy.",
    );
  }
  if privacy.uses_tracking && !privacy.shows_tracking_prompt {
    rules.add(
      "privacy.tracking-no-prompt",
      Severity::Critical,
      Category::AppPrivacy,
      "5.1.2",
      "Tracking without the App Tracking Transparency prompt",
      "The app tracks users but does not show the system tracking permission prompt.",
      "Request permission with App Tracking Transparency before tracking, and declare tracking in App Privacy.",
    );
  }
  if privacy.has_accounts && !privacy.account_deletion_in_app {
    rules.add(
      "privacy.account-deletion",
      Severity::Critical,
      Category::AppPrivacy,
      "5.1.1(v)",
      "Accounts cannot be deleted in the app",
      "Apps that let people create an account must let them start deleting it inside the app.",
      "Add an account deletion option in the app.",
    );
  }
  if !privacy.third_party_sdks.trim().is_empty() && !privacy.collects_data && !privacy.uses_tracking {
    rules.add(
      "privacy.sdk-mismatch",
      Severity::Warning,
      Category::AppPrivacy,
      "5.1.1",
      "Third-party SDKs but no data declared",
      &format!(
        "The app uses {}, yet you declared that it collects no data. Most SDKs collect at least diagnostics or device identifiers.",
        privacy.third_party_sdks.trim()
      ),
      "Check each SDK's privacy documentation and make App Privacy match what it actually collects.",
    );
  }
  if privacy.collects_data && privacy.data_types.trim().is_empty() {
    rules.add(
      "privacy.data-types-missing",
      Severity::Info,
      Category::AppPrivacy,
      "5.1.1",
      "Collected data types are not listed",
      "You said the app collects data but not which types; the AI analysis works better with them.",
      "List the data types in App Privacy.",
    );
  }
}
