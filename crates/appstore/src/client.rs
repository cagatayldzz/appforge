pub(crate) mod distribution;
pub(crate) mod reviews;

use crate::{
  model::{attributes_object, Attributes},
  App, AppInfoLocalization, AppSnapshot, Credentials, Error, ReviewDetail, ScreenshotSet, Version, VersionLocalization,
};
use serde_json::{json, Value};
use std::{
  thread,
  time::{Duration, SystemTime, UNIX_EPOCH},
};

pub const DEFAULT_BASE_URL: &str = "https://api.appstoreconnect.apple.com";
const PAGE_SIZE: usize = 200;
/// How many of the newest builds are looked at for an icon.
const ICON_BUILDS: usize = 10;
/// The largest icon file taken from the answer.
const MAX_ICON_BYTES: u64 = 2 * 1024 * 1024;

pub struct Client {
  credentials: Credentials,
  agent: ureq::Agent,
  base_url: String,
}

impl Client {
  pub fn new(credentials: Credentials) -> Self {
    Self::with_base_url(credentials, DEFAULT_BASE_URL)
  }

  /// Points the client at another server; used by tests.
  pub fn with_base_url(credentials: Credentials, base_url: &str) -> Self {
    let agent: ureq::Agent = ureq::Agent::config_builder().http_status_as_error(false).timeout_global(Some(Duration::from_secs(30))).build().into();
    Self { credentials, agent, base_url: base_url.trim_end_matches('/').to_owned() }
  }

  fn send(&self, method: &str, url: &str, body: Option<&Value>) -> Result<Value, Error> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let authorization = format!("Bearer {}", self.credentials.token(now)?);
    let network = |error: ureq::Error| Error::Network(error.to_string());
    let mut response = match (method, body) {
      ("GET", _) => self.agent.get(url).header("Authorization", &authorization).call(),
      ("PATCH", Some(body)) => self.agent.patch(url).header("Authorization", &authorization).send_json(body),
      ("DELETE", _) => self.agent.delete(url).header("Authorization", &authorization).call(),
      ("POST", Some(body)) => self.agent.post(url).header("Authorization", &authorization).send_json(body),
      _ => return Err(Error::Unexpected(format!("unsupported request {method}"))),
    }
    .map_err(network)?;
    let status = response.status().as_u16();
    let text = response.body_mut().read_to_string().map_err(network)?;
    let value: Value = if text.trim().is_empty() { Value::Null } else { serde_json::from_str(&text).unwrap_or(Value::Null) };
    if status >= 400 {
      let error = &value["errors"][0];
      let message = error["detail"].as_str().or_else(|| error["title"].as_str()).unwrap_or("No details").to_owned();
      return Err(Error::Api { status, message });
    }
    Ok(value)
  }

  fn get(&self, path: &str) -> Result<Value, Error> {
    self.send("GET", &format!("{}{path}", self.base_url), None)
  }

  /// A GET whose 404 means "does not exist yet".
  fn get_optional(&self, path: &str) -> Result<Option<Value>, Error> {
    match self.get(path) {
      Ok(value) => Ok(Some(value)),
      Err(Error::Api { status: 404, .. }) => Ok(None),
      Err(error) => Err(error),
    }
  }

  /// Every item of a collection, following `links.next`.
  fn get_all(&self, path: &str) -> Result<Vec<Value>, Error> {
    self.get_collection(path).map(|(items, _)| items)
  }

  fn get_collection(&self, path: &str) -> Result<(Vec<Value>, Vec<Value>), Error> {
    let mut items = Vec::new();
    let mut included = Vec::new();
    let mut url = format!("{}{path}", self.base_url);
    loop {
      let page = self.send("GET", &url, None)?;
      items.extend(page["data"].as_array().cloned().unwrap_or_default());
      included.extend(page["included"].as_array().into_iter().flatten().cloned());
      match page["links"]["next"].as_str() {
        // Never follow a link to another host with our token.
        Some(next) if next.strip_prefix(&self.base_url).is_some_and(|path| path.starts_with('/')) => url = next.to_owned(),
        _ => return Ok((items, included)),
      }
    }
  }

  fn create_resource(&self, kind: &str, attributes: Value, relationships: Value) -> Result<Value, Error> {
    let mut data = json!({"type": kind, "relationships": relationships});
    if !attributes.is_null() {
      data["attributes"] = attributes;
    }
    self.write("POST", &format!("/v1/{kind}"), json!({"data": data}))
  }

  fn write(&self, method: &str, path: &str, body: Value) -> Result<Value, Error> {
    self.send(method, &format!("{}{path}", self.base_url), Some(&body))
  }

  fn patch_attributes(&self, kind: &str, id: &str, attributes: Attributes) -> Result<Value, Error> {
    let body = json!({"data": {"type": kind, "id": id, "attributes": attributes_object(attributes)}});
    self.send("PATCH", &format!("{}/v1/{kind}/{id}", self.base_url), Some(&body))
  }

  /// Checks the credentials with the cheapest call there is.
  pub fn verify(&self) -> Result<(), Error> {
    self.get("/v1/apps?limit=1&fields[apps]=name").map(|_| ())
  }

  pub fn apps(&self) -> Result<Vec<App>, Error> {
    let items = self.get_all(&format!("/v1/apps?limit={PAGE_SIZE}&fields[apps]=name,bundleId,sku,primaryLocale"))?;
    Ok(items.iter().map(parse_app).collect())
  }

  /// The icon of an app as an image file about `size` pixels wide. App Store Connect serves it with the builds of the app,
  /// so an app that has no build with an icon yet has none (`None`). The newest builds are tried first: an old build
  /// that expired has no icon any more.
  pub fn app_icon(&self, app_id: &str, size: u32) -> Result<Option<Vec<u8>>, Error> {
    let path = format!("/v1/apps/{app_id}/builds?limit={ICON_BUILDS}&sort=-uploadedDate&fields[builds]=iconAssetToken,expired");
    let Some(page) = self.get_optional(&path)? else { return Ok(None) };
    let template = page["data"]
      .as_array()
      .into_iter()
      .flatten()
      .filter(|build| build["attributes"]["expired"].as_bool() != Some(true))
      .find_map(|build| build["attributes"]["iconAssetToken"]["templateUrl"].as_str());
    let Some(template) = template else { return Ok(None) };
    let url = template.replace("{w}", &size.to_string()).replace("{h}", &size.to_string()).replace("{f}", "png");
    // The address comes from the answer: it must be a secure web address, and the token is never sent to it.
    if !url.starts_with("https://") && !url.starts_with(&self.base_url) {
      return Err(Error::Unexpected("the icon address is not a secure web address".into()));
    }
    let network = |error: ureq::Error| Error::Network(error.to_string());
    let mut response = self.agent.get(&url).call().map_err(network)?;
    let status = response.status().as_u16();
    if status >= 400 {
      return Err(Error::Api { status, message: "The icon could not be downloaded".into() });
    }
    response.body_mut().with_config().limit(MAX_ICON_BYTES).read_to_vec().map(Some).map_err(network)
  }

  /// Reads everything about `app`: App Information, the listing of its current version, screenshots and review details.
  pub fn snapshot(&self, app: &App) -> Result<AppSnapshot, Error> {
    let (info, version) = thread::scope(|scope| {
      let info = scope.spawn(|| self.read_info(&app.id));
      let version = self.read_version(app);
      (info.join().unwrap_or_else(|_| Err(Error::Unexpected("reading App Information panicked".into()))), version)
    });
    let (info_state, info_localizations) = info?;
    let (version, version_localizations, screenshot_sets, review_detail) = version?;
    Ok(AppSnapshot { app: app.clone(), info_state, info_localizations, version, version_localizations, screenshot_sets, review_detail })
  }

  fn read_info(&self, app_id: &str) -> Result<(String, Vec<AppInfoLocalization>), Error> {
    let infos = self.get_all(&format!("/v1/apps/{app_id}/appInfos"))?;
    let Some(info) = pick_editable(&infos, &["appStoreState", "state"]) else { return Ok((String::new(), Vec::new())) };
    let id = info["id"].as_str().unwrap_or_default();
    let localizations = self.get_all(&format!("/v1/appInfos/{id}/appInfoLocalizations"))?;
    Ok((state_of(info, &["appStoreState", "state"]), localizations.iter().map(parse_info_localization).collect()))
  }

  #[allow(clippy::type_complexity)]
  fn read_version(&self, app: &App) -> Result<(Option<Version>, Vec<VersionLocalization>, Vec<ScreenshotSet>, Option<ReviewDetail>), Error> {
    let versions = self.get_all(&format!("/v1/apps/{}/appStoreVersions?filter[platform]=IOS&limit=20", app.id))?;
    let Some(version) = pick_editable(&versions, &["appVersionState", "appStoreState"]) else {
      return Ok((None, Vec::new(), Vec::new(), None));
    };
    let id = version["id"].as_str().unwrap_or_default().to_owned();
    let summary = Version {
      id: id.clone(),
      version_string: version["attributes"]["versionString"].as_str().unwrap_or_default().to_owned(),
      state: state_of(version, &["appVersionState", "appStoreState"]),
    };
    let localizations: Vec<VersionLocalization> =
      self.get_all(&format!("/v1/appStoreVersions/{id}/appStoreVersionLocalizations"))?.iter().map(parse_version_localization).collect();
    let primary = localizations.iter().find(|localization| localization.locale == app.primary_locale).or_else(|| localizations.first());
    let screenshot_sets = match primary {
      Some(localization) => self.read_screenshot_sets(&localization.id)?,
      None => Vec::new(),
    };
    let review_detail = self
      .get_optional(&format!("/v1/appStoreVersions/{id}/appStoreReviewDetail"))?
      .filter(|value| value["data"].is_object())
      .map(|value| parse_review_detail(&value["data"]));
    Ok((Some(summary), localizations, screenshot_sets, review_detail))
  }

  fn read_screenshot_sets(&self, localization_id: &str) -> Result<Vec<ScreenshotSet>, Error> {
    let page = self.get(&format!("/v1/appStoreVersionLocalizations/{localization_id}/appScreenshotSets?include=appScreenshots&limit=50"))?;
    let sets = page["data"].as_array().cloned().unwrap_or_default();
    Ok(
      sets
        .iter()
        .map(|set| ScreenshotSet {
          display_type: set["attributes"]["screenshotDisplayType"].as_str().unwrap_or_default().to_owned(),
          count: set["relationships"]["appScreenshots"]["data"].as_array().map_or(0, Vec::len),
        })
        .collect(),
    )
  }

  /// Sends only what changed; returns whether anything was sent.
  pub fn update_info_localization(&self, edited: &AppInfoLocalization, original: &AppInfoLocalization) -> Result<bool, Error> {
    let changes = edited.changes_from(original);
    if changes.is_empty() {
      return Ok(false);
    }
    self.patch_attributes("appInfoLocalizations", &original.id, changes)?;
    Ok(true)
  }

  /// Sends only what changed; returns whether anything was sent.
  pub fn update_version_localization(&self, edited: &VersionLocalization, original: &VersionLocalization) -> Result<bool, Error> {
    let changes = edited.changes_from(original);
    if changes.is_empty() {
      return Ok(false);
    }
    self.patch_attributes("appStoreVersionLocalizations", &original.id, changes)?;
    Ok(true)
  }

  /// Creates the review detail of `version_id` when it has none, otherwise updates it. Returns what Apple now holds.
  pub fn save_review_detail(&self, version_id: &str, detail: &ReviewDetail) -> Result<ReviewDetail, Error> {
    let answer = match &detail.id {
      Some(id) => self.patch_attributes("appStoreReviewDetails", id, detail.attributes())?,
      None => {
        let body = json!({"data": {
          "type": "appStoreReviewDetails",
          "attributes": attributes_object(detail.attributes()),
          "relationships": {"appStoreVersion": {"data": {"type": "appStoreVersions", "id": version_id}}},
        }});
        self.send("POST", &format!("{}/v1/appStoreReviewDetails", self.base_url), Some(&body))?
      }
    };
    Ok(parse_review_detail(&answer["data"]))
  }
}

fn text(value: &Value, name: &str) -> String {
  value["attributes"][name].as_str().unwrap_or_default().to_owned()
}

fn state_of(item: &Value, names: &[&str]) -> String {
  names.iter().find_map(|name| item["attributes"][*name].as_str()).unwrap_or_default().to_owned()
}

/// The item Apple still lets you edit, otherwise the first one.
fn pick_editable<'a>(items: &'a [Value], state_names: &[&str]) -> Option<&'a Value> {
  let editable = |item: &&Value| Version { state: state_of(item, state_names), ..Default::default() }.is_editable();
  items.iter().find(editable).or_else(|| items.first())
}

fn parse_app(item: &Value) -> App {
  App {
    id: item["id"].as_str().unwrap_or_default().to_owned(),
    name: text(item, "name"),
    bundle_id: text(item, "bundleId"),
    sku: text(item, "sku"),
    primary_locale: text(item, "primaryLocale"),
  }
}

fn parse_info_localization(item: &Value) -> AppInfoLocalization {
  AppInfoLocalization {
    id: item["id"].as_str().unwrap_or_default().to_owned(),
    locale: text(item, "locale"),
    name: text(item, "name"),
    subtitle: text(item, "subtitle"),
    privacy_policy_url: text(item, "privacyPolicyUrl"),
  }
}

fn parse_version_localization(item: &Value) -> VersionLocalization {
  VersionLocalization {
    id: item["id"].as_str().unwrap_or_default().to_owned(),
    locale: text(item, "locale"),
    description: text(item, "description"),
    keywords: text(item, "keywords"),
    promotional_text: text(item, "promotionalText"),
    support_url: text(item, "supportUrl"),
    marketing_url: text(item, "marketingUrl"),
    whats_new: text(item, "whatsNew"),
  }
}

fn parse_review_detail(item: &Value) -> ReviewDetail {
  ReviewDetail {
    id: item["id"].as_str().map(str::to_owned),
    contact_first_name: text(item, "contactFirstName"),
    contact_last_name: text(item, "contactLastName"),
    contact_phone: text(item, "contactPhone"),
    contact_email: text(item, "contactEmail"),
    demo_account_required: item["attributes"]["demoAccountRequired"].as_bool().unwrap_or(false),
    demo_account_name: text(item, "demoAccountName"),
    demo_account_password: text(item, "demoAccountPassword"),
    notes: text(item, "notes"),
  }
}

fn resource_id(item: &Value) -> &str {
  item["id"].as_str().unwrap_or_default()
}

fn relationship(kind: &str, id: &str) -> Value {
  json!({"data": {"type": kind, "id": id}})
}
