//! The age rating declaration of an app: what the app contains, which Apple turns into the age ratings.

use super::Client;
use crate::{
  model::{AgeRatingDeclaration, AGE_FLAG_KEYS, AGE_LEVEL_KEYS},
  Error,
};
use serde_json::Value;

/// Reads a declaration from the `ageRatingDeclarations` resource of an `appInfos` answer.
pub(super) fn parse_declaration(item: &Value) -> AgeRatingDeclaration {
  let attributes = &item["attributes"];
  let level = |key: &str| attributes[key].as_str().unwrap_or("NONE").to_owned();
  AgeRatingDeclaration {
    id: item["id"].as_str().unwrap_or_default().to_owned(),
    levels: AGE_LEVEL_KEYS.map(level),
    flags: AGE_FLAG_KEYS.map(|key| attributes[key].as_bool().unwrap_or(false)),
    age_override: level("ageRatingOverrideV2"),
    korea_override: level("koreaAgeRatingOverride"),
    info_url: attributes["developerAgeRatingInfoUrl"].as_str().unwrap_or_default().to_owned(),
  }
}

impl Client {
  /// Sends only what changed; returns whether anything was sent.
  pub fn update_age_rating(&self, edited: &AgeRatingDeclaration, original: &AgeRatingDeclaration) -> Result<bool, Error> {
    let changes = edited.changes_from(original);
    if changes.is_empty() || original.id.is_empty() {
      return Ok(false);
    }
    self.patch_attributes("ageRatingDeclarations", &original.id, changes)?;
    Ok(true)
  }
}
