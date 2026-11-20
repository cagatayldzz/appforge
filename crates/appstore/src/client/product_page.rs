//! Product page placements and general App Store information.

use super::{text, Client};
use crate::{EncryptionDeclaration, Error, GeneralInformation, ScreenshotSet};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};

#[derive(Clone, Debug)]
pub struct ProductAsset {
  pub id: String,
  pub kind: String,
  pub device: String,
  pub state: String,
  pub thumbnail: Option<String>,
  pub video: Option<String>,
}

impl Client {
  pub(super) fn submission_screenshots(&self, localization: &str) -> Result<Vec<ScreenshotSet>, Error> {
    let path = format!("/v1/appStoreVersionLocalizations/{localization}/placements?filter[placementType]=APP_SCREENSHOT&limit=50");
    let mut groups = BTreeMap::new();
    match self.get_all(&path) {
      Ok(placements) => {
        for placement in placements {
          *groups.entry(text(&placement, "placementGroup")).or_insert(0) += 1;
        }
      }
      Err(Error::Api { status: 400 | 403 | 404, .. }) => {}
      Err(error) => return Err(error),
    }
    if groups.is_empty() {
      return match self.read_screenshot_sets(localization) {
        Err(Error::Api { status: 404 | 410, .. }) => Ok(Vec::new()),
        result => result,
      };
    }
    Ok(groups.into_iter().map(|(display_type, count)| ScreenshotSet { display_type, count }).collect())
  }

  pub fn remove_product_placement(&self, id: &str) -> Result<bool, Error> {
    self.send("DELETE", &format!("{}/v1/appAssetLibraryPlacements/{id}", self.base_url), None).map(|_| true)
  }

  pub fn product_assets(&self, localization: &str) -> Result<Vec<ProductAsset>, Error> {
    let path = format!("/v1/appStoreVersionLocalizations/{localization}/placements?include=image,video&sort=placementGroupPosition&limit=50");
    let (placements, included) = self.get_collection(&path)?;
    let assets: HashMap<_, _> = included.iter().filter_map(|item| Some((item["id"].as_str()?, item))).collect();
    Ok(
      placements
        .iter()
        .filter_map(|placement| {
          let id = placement["id"].as_str()?;
          let image = placement.pointer("/relationships/image/data/id").and_then(Value::as_str).and_then(|id| assets.get(id));
          let video = placement.pointer("/relationships/video/data/id").and_then(Value::as_str).and_then(|id| assets.get(id));
          let thumbnail = image
            .map(|item| &item["attributes"]["imageAsset"])
            .or_else(|| video.map(|item| &item["attributes"]["previewFrameImage"]["image"]))
            .and_then(thumbnail);
          Some(ProductAsset {
            id: id.into(),
            kind: text(placement, "placementType"),
            device: text(placement, "placementGroup"),
            state: text(placement, "state"),
            thumbnail,
            video: video.and_then(|item| item["attributes"]["videoAsset"].as_str()).filter(|url| url.starts_with("https://")).map(str::to_owned),
          })
        })
        .collect(),
    )
  }
}

fn thumbnail(image: &Value) -> Option<String> {
  let template = image["templateUrl"].as_str()?.strip_prefix("https://")?;
  let width = image["width"].as_u64()?.max(1);
  let height = image["height"].as_u64()?.max(1);
  let longest = width.max(height);
  let size = |dimension: u64| ((dimension as f64 / longest as f64 * 640.0).round() as u64).max(1).to_string();
  Some(format!("https://{}", template.replace("{w}", &size(width)).replace("{h}", &size(height)).replace("{f}", "jpg")))
}

pub(super) fn parse_encryption(item: &Value) -> EncryptionDeclaration {
  let flag = |name: &str| item["attributes"][name].as_bool().unwrap_or_default();
  EncryptionDeclaration {
    state: text(item, "appEncryptionDeclarationState"),
    created: text(item, "createdDate"),
    exempt: flag("exempt"),
    proprietary: flag("containsProprietaryCryptography"),
    third_party: flag("containsThirdPartyCryptography"),
    french_store: flag("availableOnFrenchStore"),
  }
}

pub(super) fn general_information(info: &Value, included: &[Value]) -> GeneralInformation {
  let categories =
    ["primaryCategory", "primarySubcategoryOne", "primarySubcategoryTwo", "secondaryCategory", "secondarySubcategoryOne", "secondarySubcategoryTwo"]
      .map(|key| info["relationships"][key]["data"]["id"].as_str().unwrap_or_default().to_owned());
  let ratings: Vec<_> = [
    ("Global", "appStoreAgeRating"),
    ("Australia", "australiaAgeRating"),
    ("Brazil", "brazilAgeRatingV2"),
    ("France", "franceAgeRating"),
    ("South Korea", "koreaAgeRating"),
  ]
  .into_iter()
  .filter_map(|(label, key)| Some((label.to_owned(), info["attributes"][key].as_str()?.to_owned())))
  .collect();
  let declaration = &info["relationships"]["ageRatingDeclaration"]["data"];
  let age_declaration =
    included.iter().find(|item| item["type"] == declaration["type"] && item["id"] == declaration["id"]).map(super::age_rating::parse_declaration);
  let age_suitability_url = age_declaration.as_ref().map_or_else(String::new, |item| item.info_url.clone());
  GeneralInformation {
    id: info["id"].as_str().unwrap_or_default().into(),
    categories,
    ratings,
    age_suitability_url,
    age_declaration,
    ..Default::default()
  }
}

impl Client {
  pub(super) fn territory_age_ratings(&self, info: &str) -> Result<Vec<(String, String)>, Error> {
    let path = format!("/v1/appInfos/{info}/territoryAgeRatings?include=territory&limit=200");
    Ok(
      self
        .get_all(&path)?
        .iter()
        .filter_map(|item| {
          let territory = item.pointer("/relationships/territory/data/id")?.as_str()?;
          let rating = item.pointer("/attributes/appStoreAgeRating")?.as_str()?;
          Some((territory.to_owned(), rating.to_owned()))
        })
        .collect(),
    )
  }

  pub fn download_product_thumbnail(&self, url: &str) -> Result<Vec<u8>, Error> {
    if !url.starts_with("https://") {
      return Err(Error::Unexpected("The thumbnail address must use HTTPS.".into()));
    }
    let network = |error: ureq::Error| Error::Network(error.to_string());
    let mut response = self.agent.get(url).call().map_err(network)?;
    if !response.status().is_success() {
      return Err(Error::Api { status: response.status().as_u16(), message: "The thumbnail could not be downloaded.".into() });
    }
    response.body_mut().with_config().limit(2 * 1024 * 1024).read_to_vec().map_err(network)
  }
}

impl Client {
  /// Ensures both localization records exist. A retry after a partial write creates only the missing record.
  pub fn add_localization(&self, info: &str, version: &str, locale: &str, name: &str, privacy_url: &str) -> Result<bool, Error> {
    let info_items = self.get_all(&format!("/v1/appInfos/{info}/appInfoLocalizations"))?;
    let version_items = self.get_all(&format!("/v1/appStoreVersions/{version}/appStoreVersionLocalizations"))?;
    let missing_info = !info_items.iter().any(|item| item["attributes"]["locale"] == locale);
    let missing_version = !version_items.iter().any(|item| item["attributes"]["locale"] == locale);
    if missing_info {
      self.create_resource(
        "appInfoLocalizations",
        json!({"locale": locale, "name": name, "privacyPolicyUrl": privacy_url}),
        json!({"appInfo": {"data": {"type": "appInfos", "id": info}}}),
      )?;
    }
    if missing_version {
      self.create_resource(
        "appStoreVersionLocalizations",
        json!({"locale": locale}),
        json!({"appStoreVersion": {"data": {"type": "appStoreVersions", "id": version}}}),
      )?;
    }
    Ok(missing_info || missing_version)
  }
}
