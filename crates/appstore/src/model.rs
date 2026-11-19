//! The plain data the client returns: apps, versions, localizations, review details.

use serde_json::{Map, Value};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct App {
  pub id: String,
  pub name: String,
  pub bundle_id: String,
  pub sku: String,
  pub primary_locale: String,
}

/// The name, subtitle and privacy policy of one locale (the "App Information" page).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AppInfoLocalization {
  pub id: String,
  pub locale: String,
  pub name: String,
  pub subtitle: String,
  pub privacy_policy_url: String,
}

/// The store listing text of one locale and version.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct VersionLocalization {
  pub id: String,
  pub locale: String,
  pub description: String,
  pub keywords: String,
  pub promotional_text: String,
  pub support_url: String,
  pub marketing_url: String,
  pub whats_new: String,
}

/// What Apple's reviewers get: contact details, a demo account and notes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ReviewDetail {
  /// `None` until the review detail exists on Apple's side.
  pub id: Option<String>,
  pub contact_first_name: String,
  pub contact_last_name: String,
  pub contact_phone: String,
  pub contact_email: String,
  pub demo_account_required: bool,
  pub demo_account_name: String,
  pub demo_account_password: String,
  pub notes: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Version {
  pub id: String,
  pub version_string: String,
  pub state: String,
}

impl Version {
  /// Whether Apple still accepts edits to this version's listing and review information.
  pub fn is_editable(&self) -> bool {
    matches!(self.state.as_str(), "PREPARE_FOR_SUBMISSION" | "DEVELOPER_REJECTED" | "REJECTED" | "METADATA_REJECTED" | "INVALID_BINARY")
  }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScreenshotSet {
  pub display_type: String,
  pub count: usize,
}

/// Everything AppForge shows and analyses for one app, read in one go.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AppSnapshot {
  pub app: App,
  pub info_state: String,
  pub info_localizations: Vec<AppInfoLocalization>,
  pub version: Option<Version>,
  pub version_localizations: Vec<VersionLocalization>,
  /// Screenshot sets of the primary locale of the version.
  pub screenshot_sets: Vec<ScreenshotSet>,
  pub review_detail: Option<ReviewDetail>,
}

pub(crate) type Attributes = Vec<(&'static str, Value)>;

fn changed(attributes: &mut Attributes, name: &'static str, before: &str, after: &str) {
  if before != after {
    attributes.push((name, Value::String(after.to_owned())));
  }
}

impl AppInfoLocalization {
  /// The attributes of `self` that differ from `original`, named as App Store Connect names them.
  pub(crate) fn changes_from(&self, original: &Self) -> Attributes {
    let mut attributes = Attributes::new();
    changed(&mut attributes, "name", &original.name, &self.name);
    changed(&mut attributes, "subtitle", &original.subtitle, &self.subtitle);
    changed(&mut attributes, "privacyPolicyUrl", &original.privacy_policy_url, &self.privacy_policy_url);
    attributes
  }
}

impl VersionLocalization {
  pub(crate) fn changes_from(&self, original: &Self) -> Attributes {
    let mut attributes = Attributes::new();
    changed(&mut attributes, "description", &original.description, &self.description);
    changed(&mut attributes, "keywords", &original.keywords, &self.keywords);
    changed(&mut attributes, "promotionalText", &original.promotional_text, &self.promotional_text);
    changed(&mut attributes, "supportUrl", &original.support_url, &self.support_url);
    changed(&mut attributes, "marketingUrl", &original.marketing_url, &self.marketing_url);
    changed(&mut attributes, "whatsNew", &original.whats_new, &self.whats_new);
    attributes
  }
}

impl ReviewDetail {
  /// Every attribute, used to create the review detail and to update it.
  pub(crate) fn attributes(&self) -> Attributes {
    vec![
      ("contactFirstName", self.contact_first_name.clone().into()),
      ("contactLastName", self.contact_last_name.clone().into()),
      ("contactPhone", self.contact_phone.clone().into()),
      ("contactEmail", self.contact_email.clone().into()),
      ("demoAccountRequired", self.demo_account_required.into()),
      ("demoAccountName", self.demo_account_name.clone().into()),
      ("demoAccountPassword", self.demo_account_password.clone().into()),
      ("notes", self.notes.clone().into()),
    ]
  }
}

pub(crate) fn attributes_object(attributes: Attributes) -> Value {
  Value::Object(attributes.into_iter().map(|(name, value)| (name.to_owned(), value)).collect::<Map<_, _>>())
}
