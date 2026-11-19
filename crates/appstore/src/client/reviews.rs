//! Customer reviews and developer responses.

use super::{relationship, resource_id, text, Client, Error};
use serde_json::json;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct CustomerReview {
  pub id: String,
  pub title: String,
  pub body: String,
  pub author: String,
  pub territory: String,
  pub created: String,
  pub rating: u64,
  pub response: Option<ReviewResponse>,
}

#[derive(Clone, Debug)]
pub struct ReviewResponse {
  pub id: String,
  pub body: String,
  pub state: String,
}

impl Client {
  pub fn customer_reviews(&self, app: &str) -> Result<Vec<CustomerReview>, Error> {
    self.reviews_at(&format!("/v1/apps/{app}/customerReviews"))
  }

  pub fn version_customer_reviews(&self, version: &str) -> Result<Vec<CustomerReview>, Error> {
    self.reviews_at(&format!("/v1/appStoreVersions/{version}/customerReviews"))
  }

  pub fn review_versions(&self, app: &str) -> Result<Vec<(String, String)>, Error> {
    let versions = self.get_all(&format!("/v1/apps/{app}/appStoreVersions?filter[platform]=IOS&fields[appStoreVersions]=versionString&limit=200"))?;
    Ok(versions.iter().map(|item| (resource_id(item).to_owned(), text(item, "versionString"))).collect())
  }

  fn reviews_at(&self, path: &str) -> Result<Vec<CustomerReview>, Error> {
    let (reviews, included) = self.get_collection(&format!("{path}?include=response&sort=-createdDate&limit=200"))?;
    let responses: HashMap<_, _> =
      included.iter().filter(|item| item["type"].as_str() == Some("customerReviewResponses")).map(|item| (resource_id(item), item)).collect();
    Ok(
      reviews
        .iter()
        .map(|item| {
          let response = item["relationships"]["response"]["data"]["id"].as_str().and_then(|id| responses.get(id)).map(|item| ReviewResponse {
            id: resource_id(item).to_owned(),
            body: text(item, "responseBody"),
            state: text(item, "state"),
          });
          CustomerReview {
            id: resource_id(item).to_owned(),
            title: text(item, "title"),
            body: text(item, "body"),
            author: text(item, "reviewerNickname"),
            territory: text(item, "territory"),
            created: text(item, "createdDate"),
            rating: item["attributes"]["rating"].as_u64().unwrap_or_default(),
            response,
          }
        })
        .collect(),
    )
  }

  pub fn save_customer_review_response(&self, review: &str, body: &str) -> Result<(), Error> {
    if body.trim().is_empty() {
      return Err(Error::Unexpected("Enter a response before publishing.".into()));
    }
    self.create_resource("customerReviewResponses", json!({"responseBody": body}), json!({"review": relationship("customerReviews", review)}))?;
    Ok(())
  }

  pub fn delete_customer_review_response(&self, response: &str) -> Result<(), Error> {
    self.send("DELETE", &format!("{}/v1/customerReviewResponses/{response}", self.base_url), None)?;
    Ok(())
  }
}
