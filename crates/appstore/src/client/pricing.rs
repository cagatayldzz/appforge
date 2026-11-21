//! Pricing and availability: the price schedule, the price of every territory, the countries and regions the app is sold in.

use super::{relationship, resource_id, text, Client, Error, PAGE_SIZE};
use serde_json::{json, Value};
use std::collections::HashMap;

/// One entry of the base territory's price schedule: a price from a date on (an empty start means "from the beginning").
#[derive(Clone, Debug, Default)]
pub struct PricePeriod {
  pub start: String,
  pub end: String,
  pub price: String,
  pub proceeds: String,
  pub point: String,
}

/// What a territory costs right now, set by hand (the base territory) or worked out by Apple from it.
#[derive(Clone, Debug, Default)]
pub struct TerritoryPrice {
  pub territory: String,
  pub currency: String,
  pub price: String,
  pub proceeds: String,
  pub manual: bool,
}

#[derive(Clone, Debug, Default)]
pub struct TerritoryAvailability {
  pub id: String,
  pub territory: String,
  pub currency: String,
  pub available: bool,
  pub release_date: String,
  pub statuses: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct Availability {
  pub id: String,
  pub new_territories: bool,
  pub territories: Vec<TerritoryAvailability>,
}

/// A price the app can be sold at in one territory.
#[derive(Clone, Debug, Default)]
pub struct PricePoint {
  pub id: String,
  pub price: String,
  pub proceeds: String,
}

#[derive(Clone, Debug, Default)]
pub struct Pricing {
  pub base: Option<String>,
  pub base_currency: String,
  pub periods: Vec<PricePeriod>,
  pub prices: Vec<TerritoryPrice>,
  pub availability: Option<Availability>,
}

impl Pricing {
  /// The territories where the app is for sale.
  pub fn available_count(&self) -> usize {
    self.availability.as_ref().map_or(0, |availability| availability.territories.iter().filter(|item| item.available).count())
  }
}

/// `type`/`id` pairs of a response's `included`, to look a related resource up.
fn index(included: &[Value]) -> HashMap<(String, String), &Value> {
  included.iter().map(|item| ((text_of(item, "type"), resource_id(item).to_owned()), item)).collect()
}

fn text_of(item: &Value, key: &str) -> String {
  item[key].as_str().unwrap_or_default().to_owned()
}

fn related(item: &Value, name: &str) -> String {
  item["relationships"][name]["data"]["id"].as_str().unwrap_or_default().to_owned()
}

impl Client {
  pub fn pricing(&self, app: &str) -> Result<Pricing, Error> {
    let mut pricing = Pricing::default();
    if let Some(schedule) = self.get_optional(&format!("/v1/apps/{app}/appPriceSchedule?include=baseTerritory"))? {
      let schedule_id = resource_id(&schedule["data"]).to_owned();
      let included = schedule["included"].as_array().cloned().unwrap_or_default();
      if let Some(territory) = included.iter().find(|item| item["type"] == "territories") {
        pricing.base = Some(resource_id(territory).to_owned());
        pricing.base_currency = text(territory, "currency");
      }
      let query =
        format!("limit={PAGE_SIZE}&include=appPricePoint,territory&fields[appPricePoints]=customerPrice,proceeds&fields[territories]=currency");
      let (manual, manual_included) = self.get_collection(&format!("/v1/appPriceSchedules/{schedule_id}/manualPrices?{query}"))?;
      let (automatic, automatic_included) = self.get_collection(&format!("/v1/appPriceSchedules/{schedule_id}/automaticPrices?{query}"))?;
      for (items, included, is_manual) in [(&manual, &manual_included, true), (&automatic, &automatic_included, false)] {
        let lookup = index(included);
        for item in items {
          let point = lookup.get(&("appPricePoints".to_owned(), related(item, "appPricePoint")));
          let territory = related(item, "territory");
          let currency = lookup.get(&("territories".to_owned(), territory.clone())).map(|found| text(found, "currency")).unwrap_or_default();
          let (price, proceeds) = point.map(|found| (text(found, "customerPrice"), text(found, "proceeds"))).unwrap_or_default();
          let start = text(item, "startDate");
          if is_manual {
            pricing.periods.push(PricePeriod {
              start: start.clone(),
              end: text(item, "endDate"),
              price: price.clone(),
              proceeds: proceeds.clone(),
              point: related(item, "appPricePoint"),
            });
          }
          if start.is_empty() {
            pricing.prices.push(TerritoryPrice { territory, currency, price, proceeds, manual: is_manual });
          }
        }
      }
      pricing.periods.sort_by(|a, b| a.start.cmp(&b.start));
      pricing.prices.sort_by(|a, b| a.territory.cmp(&b.territory));
    }
    if let Some(found) = self.get_optional(&format!("/v1/apps/{app}/appAvailabilityV2"))? {
      let id = resource_id(&found["data"]).to_owned();
      let (items, included) =
        self.get_collection(&format!("/v2/appAvailabilities/{id}/territoryAvailabilities?limit={PAGE_SIZE}&include=territory"))?;
      let lookup = index(&included);
      let mut territories: Vec<TerritoryAvailability> = items
        .iter()
        .map(|item| {
          let territory = related(item, "territory");
          let currency = lookup.get(&("territories".to_owned(), territory.clone())).map(|found| text(found, "currency")).unwrap_or_default();
          TerritoryAvailability {
            id: resource_id(item).to_owned(),
            territory,
            currency,
            available: item["attributes"]["available"].as_bool().unwrap_or(false),
            release_date: text(item, "releaseDate"),
            statuses: item["attributes"]["contentStatuses"]
              .as_array()
              .map(|list| list.iter().filter_map(|status| status.as_str().map(str::to_owned)).collect())
              .unwrap_or_default(),
          }
        })
        .collect();
      territories.sort_by(|a, b| a.territory.cmp(&b.territory));
      pricing.availability =
        Some(Availability { id, new_territories: found["data"]["attributes"]["availableInNewTerritories"].as_bool().unwrap_or(false), territories });
    }
    Ok(pricing)
  }

  /// The prices the app can be sold at in `territory`, cheapest first.
  pub fn price_points(&self, app: &str, territory: &str) -> Result<Vec<PricePoint>, Error> {
    let items = self.get_all(&format!(
      "/v1/apps/{app}/appPricePoints?limit={PAGE_SIZE}&filter[territory]={territory}&fields[appPricePoints]=customerPrice,proceeds"
    ))?;
    let mut points: Vec<PricePoint> = items
      .iter()
      .map(|item| PricePoint { id: resource_id(item).to_owned(), price: text(item, "customerPrice"), proceeds: text(item, "proceeds") })
      .collect();
    points.sort_by(|a, b| a.price.parse::<f64>().unwrap_or(0.0).total_cmp(&b.price.parse::<f64>().unwrap_or(0.0)));
    Ok(points)
  }

  /// Sets the price in `base` to the price point `point`. `start` is the first day of the new price; without it the price applies
  /// now. A new schedule replaces the old one, so the entries that must survive (`keep`) are sent again.
  pub fn set_price(&self, app: &str, base: &str, point: &str, start: Option<&str>, keep: &[PricePeriod]) -> Result<(), Error> {
    let mut included = Vec::new();
    let mut manual = Vec::new();
    let mut entries: Vec<(String, Option<&str>, Option<&str>)> = keep
      .iter()
      .map(|period| {
        (period.point.clone(), (!period.start.is_empty()).then_some(period.start.as_str()), (!period.end.is_empty()).then_some(period.end.as_str()))
      })
      .collect();
    entries.push((point.to_owned(), start, None));
    for (index, (point, start, end)) in entries.iter().enumerate() {
      let id = format!("${{price{index}}}");
      manual.push(json!({"type": "appPrices", "id": id}));
      included.push(json!({
        "type": "appPrices",
        "id": id,
        "attributes": {"startDate": start, "endDate": end},
        "relationships": {"appPricePoint": relationship("appPricePoints", point)},
      }));
    }
    let body = json!({
      "data": {
        "type": "appPriceSchedules",
        "relationships": {
          "app": relationship("apps", app),
          "baseTerritory": relationship("territories", base),
          "manualPrices": {"data": manual},
        },
      },
      "included": included,
    });
    self.write("POST", "/v1/appPriceSchedules", body)?;
    Ok(())
  }

  /// Puts the app on sale in a territory or takes it off.
  pub fn set_territory_available(&self, id: &str, available: bool) -> Result<(), Error> {
    self.patch_attributes("territoryAvailabilities", id, vec![("available", json!(available))])?;
    Ok(())
  }
}
