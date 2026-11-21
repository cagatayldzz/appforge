//! Versions, builds, review submissions, release type and phased release.

use super::{relationship, resource_id, state_of, text, Client, Error};
use serde_json::{json, Value};

#[derive(Debug)]
pub struct StoreVersion {
  pub id: String,
  pub number: String,
  pub state: String,
  pub release_type: String,
  pub release_date: String,
  /// When the version was created, which App Store Connect's History lists as "Prepare for Submission".
  pub created: String,
}

#[derive(Debug)]
pub struct StoreBuild {
  pub id: String,
  pub number: String,
  pub state: String,
  pub expired: bool,
  pub internal_only: bool,
  /// The export compliance answer of the build; `None` while it is not answered.
  pub uses_non_exempt_encryption: Option<bool>,
}

#[derive(Debug)]
pub struct Distribution {
  pub versions: Vec<StoreVersion>,
  pub builds: Vec<StoreBuild>,
  pub selected_build: Option<String>,
  pub phased_release: Option<(String, String)>,
  pub submissions: Vec<ReviewSubmission>,
}

#[derive(Debug)]
pub struct ReviewSubmission {
  pub id: String,
  pub state: String,
  pub date: String,
  pub item_count: usize,
  pub includes_version: bool,
  /// The versions in the submission, such as `1.0`, and who submitted it (empty when Apple does not say).
  pub versions: Vec<String>,
  pub submitted_by: String,
  /// Every item of the submission was removed from it, which App Store Connect shows as "Removed".
  pub removed: bool,
}

impl Client {
  pub fn distribution(&self, app: &str, version: Option<(&str, &str)>) -> Result<Distribution, Error> {
    let versions = self.get_all(&format!("/v1/apps/{app}/appStoreVersions?filter[platform]=IOS&limit=200"))?;
    let versions = versions
      .iter()
      .map(|item| StoreVersion {
        id: resource_id(item).to_owned(),
        number: text(item, "versionString"),
        state: state_of(item, &["appVersionState", "appStoreState"]),
        release_type: text(item, "releaseType"),
        release_date: text(item, "earliestReleaseDate"),
        created: text(item, "createdDate"),
      })
      .collect();

    let mut distribution = Distribution { versions, builds: Vec::new(), selected_build: None, phased_release: None, submissions: Vec::new() };
    let Some((id, number)) = version else { return Ok(distribution) };
    let builds = self.get_all(&format!(
      "/v1/builds?filter[app]={app}&filter[preReleaseVersion.platform]=IOS&filter[preReleaseVersion.version]={number}&sort=-uploadedDate&limit=200"
    ))?;
    distribution.builds = builds
      .iter()
      .map(|item| StoreBuild {
        id: resource_id(item).to_owned(),
        number: text(item, "version"),
        state: text(item, "processingState"),
        expired: item["attributes"]["expired"].as_bool().unwrap_or(false),
        internal_only: item["attributes"]["buildAudienceType"].as_str() == Some("INTERNAL_ONLY"),
        uses_non_exempt_encryption: item["attributes"]["usesNonExemptEncryption"].as_bool(),
      })
      .collect();
    distribution.selected_build =
      self.get_optional(&format!("/v1/appStoreVersions/{id}/build"))?.and_then(|response| response["data"]["id"].as_str().map(str::to_owned));
    distribution.phased_release = self.get_optional(&format!("/v1/appStoreVersions/{id}/appStoreVersionPhasedRelease"))?.and_then(|response| {
      response["data"].as_object().map(|_| (resource_id(&response["data"]).to_owned(), text(&response["data"], "phasedReleaseState")))
    });
    let (submissions, included) = self.get_collection(&format!(
      "/v1/apps/{app}/reviewSubmissions?filter[platform]=IOS&include=items,appStoreVersionForReview,submittedByActor&limit[items]=50&limit=200"
    ))?;
    let included_by = |kind: &str, id: &str| included.iter().find(|item| item["type"] == kind && item["id"] == id);
    for item in &submissions {
      let submission = resource_id(item);
      let state = text(item, "state");
      let count = item["relationships"]["items"]["data"].as_array().map_or(0, Vec::len);
      let linked = item["relationships"]["appStoreVersionForReview"]["data"]["id"].as_str();
      let includes_version = if linked.is_none() && count > 0 && state == "READY_FOR_REVIEW" {
        let items = self.get_all(&format!("/v1/reviewSubmissions/{submission}/items?include=appStoreVersion&limit=50"))?;
        items.iter().any(|item| item["relationships"]["appStoreVersion"]["data"]["id"].as_str() == Some(id))
      } else {
        linked == Some(id)
      };
      let mut version_ids: Vec<&str> = linked.into_iter().collect();
      let mut removed_items = 0;
      for entry in item["relationships"]["items"]["data"].as_array().into_iter().flatten() {
        let version = entry["id"].as_str().and_then(|id| included_by("reviewSubmissionItems", id));
        removed_items += usize::from(version.is_some_and(|item| item["attributes"]["state"] == "REMOVED"));
        version_ids.extend(version.and_then(|item| item["relationships"]["appStoreVersion"]["data"]["id"].as_str()));
      }
      let mut versions: Vec<String> = Vec::new();
      for id in version_ids {
        let number = distribution.versions.iter().find(|version| version.id == id).map(|version| version.number.clone());
        if let Some(number) = number.filter(|number| !versions.contains(number)) {
          versions.push(number);
        }
      }
      let submitted_by = item["relationships"]["submittedByActor"]["data"]["id"]
        .as_str()
        .and_then(|id| included_by("actors", id))
        .map(|actor| format!("{} {}", text(actor, "userFirstName"), text(actor, "userLastName")).trim().to_owned())
        .unwrap_or_default();
      distribution.submissions.push(ReviewSubmission {
        id: submission.to_owned(),
        state,
        date: text(item, "submittedDate"),
        item_count: count,
        includes_version,
        versions,
        submitted_by,
        removed: count > 0 && removed_items == count,
      });
    }
    Ok(distribution)
  }

  pub fn create_ios_version(&self, app: &str, number: &str) -> Result<(), Error> {
    if !(2..=3).contains(&number.split('.').count())
      || !number.split('.').all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
    {
      return Err(Error::Unexpected("Enter a version number such as 1.2 or 1.2.1.".into()));
    }
    self.create_resource(
      "appStoreVersions",
      json!({"platform": "IOS", "versionString": number, "releaseType": "MANUAL"}),
      json!({"app": relationship("apps", app)}),
    )?;
    Ok(())
  }

  /// Answers the export compliance question of a build: whether it uses encryption that is not exempt.
  pub fn set_build_encryption(&self, build: &str, uses_non_exempt: bool) -> Result<(), Error> {
    self.patch_attributes("builds", build, vec![("usesNonExemptEncryption", Value::Bool(uses_non_exempt))]).map(|_| ())
  }

  pub fn select_version_build(&self, version: &str, build: &str) -> Result<(), Error> {
    self.write("PATCH", &format!("/v1/appStoreVersions/{version}/relationships/build"), relationship("builds", build))?;
    Ok(())
  }

  pub fn set_release_type(&self, version: &str, date: Option<&str>, automatic: bool) -> Result<(), Error> {
    let release_type = match (date, automatic) {
      (Some(_), _) => "SCHEDULED",
      (None, true) => "AFTER_APPROVAL",
      (None, false) => "MANUAL",
    };
    let mut attributes = vec![("releaseType", json!(release_type))];
    if let Some(date) = date {
      attributes.push(("earliestReleaseDate", json!(date)));
    }
    self.patch_attributes("appStoreVersions", version, attributes)?;
    Ok(())
  }

  pub fn enable_phased_release(&self, version: &str) -> Result<(), Error> {
    self.create_resource(
      "appStoreVersionPhasedReleases",
      json!({"phasedReleaseState": "INACTIVE"}),
      json!({"appStoreVersion": relationship("appStoreVersions", version)}),
    )?;
    Ok(())
  }

  pub fn set_phased_release_state(&self, id: &str, state: &str) -> Result<(), Error> {
    self.patch_attributes("appStoreVersionPhasedReleases", id, vec![("phasedReleaseState", json!(state))])?;
    Ok(())
  }

  pub fn add_version_for_review(&self, app: &str, version: &str, draft: Option<&str>) -> Result<(), Error> {
    let created;
    let submission = match draft {
      Some(id) => id,
      None => {
        created = self.create_resource("reviewSubmissions", json!({"platform": "IOS"}), json!({"app": relationship("apps", app)}))?;
        created["data"]["id"].as_str().ok_or_else(|| Error::Unexpected("Apple did not return a submission ID.".into()))?
      }
    };
    self.create_resource(
      "reviewSubmissionItems",
      Value::Null,
      json!({
        "reviewSubmission": relationship("reviewSubmissions", submission),
        "appStoreVersion": relationship("appStoreVersions", version),
      }),
    )?;
    Ok(())
  }

  pub fn submit_for_review(&self, id: &str) -> Result<(), Error> {
    self.patch_attributes("reviewSubmissions", id, vec![("submitted", json!(true))])?;
    Ok(())
  }

  pub fn cancel_review(&self, id: &str) -> Result<(), Error> {
    self.patch_attributes("reviewSubmissions", id, vec![("canceled", json!(true))])?;
    Ok(())
  }

  pub fn release_version(&self, version: &str) -> Result<(), Error> {
    self.create_resource(
      "appStoreVersionReleaseRequests",
      Value::Null,
      json!({
        "appStoreVersion": relationship("appStoreVersions", version),
      }),
    )?;
    Ok(())
  }
}
