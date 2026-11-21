mod common;

use appforge_appstore::{territory_code, territory_name, PricePeriod};
use common::{client, serve};

const SCHEDULE: &str =
  r#"{"data":{"type":"appPriceSchedules","id":"s1"},"included":[{"type":"territories","id":"USA","attributes":{"currency":"USD"}}]}"#;
const MANUAL: &str = r#"{"data":[
  {"type":"appPrices","id":"m1","attributes":{"startDate":null,"endDate":"2026-12-31","manual":true},"relationships":{"appPricePoint":{"data":{"type":"appPricePoints","id":"p0"}},"territory":{"data":{"type":"territories","id":"USA"}}}},
  {"type":"appPrices","id":"m2","attributes":{"startDate":"2027-01-01","endDate":null,"manual":true},"relationships":{"appPricePoint":{"data":{"type":"appPricePoints","id":"p1"}},"territory":{"data":{"type":"territories","id":"USA"}}}}],
 "included":[
  {"type":"appPricePoints","id":"p0","attributes":{"customerPrice":"0.99","proceeds":"0.70"}},
  {"type":"appPricePoints","id":"p1","attributes":{"customerPrice":"1.99","proceeds":"1.40"}},
  {"type":"territories","id":"USA","attributes":{"currency":"USD"}}]}"#;
const AUTOMATIC: &str = r#"{"data":[
  {"type":"appPrices","id":"a1","attributes":{"startDate":null,"endDate":null,"manual":false},"relationships":{"appPricePoint":{"data":{"type":"appPricePoints","id":"p2"}},"territory":{"data":{"type":"territories","id":"GBR"}}}}],
 "included":[
  {"type":"appPricePoints","id":"p2","attributes":{"customerPrice":"0.99","proceeds":"0.55"}},
  {"type":"territories","id":"GBR","attributes":{"currency":"GBP"}}]}"#;
const AVAILABILITY: &str = r#"{"data":{"type":"appAvailabilities","id":"av1","attributes":{"availableInNewTerritories":true}}}"#;
const TERRITORIES: &str = r#"{"data":[
  {"type":"territoryAvailabilities","id":"t1","attributes":{"available":true,"releaseDate":"2026-11-01","contentStatuses":["AVAILABLE"]},"relationships":{"territory":{"data":{"type":"territories","id":"USA"}}}},
  {"type":"territoryAvailabilities","id":"t2","attributes":{"available":false,"contentStatuses":["CANNOT_SELL"]},"relationships":{"territory":{"data":{"type":"territories","id":"CHN"}}}}],
 "included":[{"type":"territories","id":"USA","attributes":{"currency":"USD"}},{"type":"territories","id":"CHN","attributes":{"currency":"CNY"}}]}"#;

#[test]
fn pricing_reads_the_schedule_the_prices_and_the_countries_on_sale() {
  let (url, _) = serve(&[
    ("GET /v1/apps/100/appPriceSchedule", 200, SCHEDULE),
    ("GET /v1/appPriceSchedules/s1/manualPrices", 200, MANUAL),
    ("GET /v1/appPriceSchedules/s1/automaticPrices", 200, AUTOMATIC),
    ("GET /v1/apps/100/appAvailabilityV2", 200, AVAILABILITY),
    ("GET /v2/appAvailabilities/av1/territoryAvailabilities", 200, TERRITORIES),
  ]);
  let pricing = client(&url).pricing("100").unwrap();
  assert_eq!(pricing.base.as_deref(), Some("USA"));
  assert_eq!(pricing.base_currency, "USD");
  assert_eq!(pricing.periods.len(), 2);
  assert_eq!(pricing.periods[1].start, "2027-01-01");
  assert_eq!(pricing.periods[1].price, "1.99");
  // Only what applies now is a territory's price: the base price and Apple's own for the others.
  let prices: Vec<_> = pricing.prices.iter().map(|item| (item.territory.as_str(), item.price.as_str(), item.manual)).collect();
  assert_eq!(prices, [("GBR", "0.99", false), ("USA", "0.99", true)]);
  let availability = pricing.availability.unwrap();
  assert!(availability.new_territories);
  assert_eq!(availability.territories.len(), 2);
  assert!(!availability.territories[0].available && availability.territories[0].territory == "CHN");
  assert_eq!(availability.territories[1].release_date, "2026-11-01");
}

#[test]
fn an_app_without_a_schedule_or_availability_reads_as_empty() {
  let (url, _) = serve(&[]);
  let pricing = client(&url).pricing("100").unwrap();
  assert!(pricing.base.is_none() && pricing.availability.is_none() && pricing.prices.is_empty());
  assert_eq!(pricing.available_count(), 0);
}

#[test]
fn setting_a_price_sends_a_new_schedule_that_keeps_the_future_prices() {
  let (url, seen) = serve(&[("POST /v1/appPriceSchedules", 201, r#"{"data":{"type":"appPriceSchedules","id":"s2"}}"#)]);
  let future = PricePeriod { start: "2027-01-01".into(), end: String::new(), price: "1.99".into(), proceeds: "1.40".into(), point: "p1".into() };
  client(&url).set_price("100", "USA", "p9", None, &[future]).unwrap();
  let body: serde_json::Value = serde_json::from_str(&seen.lock().unwrap()[0].body).unwrap();
  assert_eq!(body["data"]["relationships"]["baseTerritory"]["data"]["id"], "USA");
  assert_eq!(body["data"]["relationships"]["app"]["data"]["id"], "100");
  let manual = body["data"]["relationships"]["manualPrices"]["data"].as_array().unwrap();
  assert_eq!(manual.len(), 2);
  let included = body["included"].as_array().unwrap();
  assert_eq!(included[0]["relationships"]["appPricePoint"]["data"]["id"], "p1");
  assert_eq!(included[0]["attributes"]["startDate"], "2027-01-01");
  assert_eq!(included[1]["relationships"]["appPricePoint"]["data"]["id"], "p9");
  assert!(included[1]["attributes"]["startDate"].is_null());
  assert_eq!(manual[1]["id"], included[1]["id"]);
}

#[test]
fn price_points_come_cheapest_first_and_a_territory_can_be_put_on_sale() {
  let points = r#"{"data":[
    {"type":"appPricePoints","id":"b","attributes":{"customerPrice":"10.99","proceeds":"7.7"}},
    {"type":"appPricePoints","id":"a","attributes":{"customerPrice":"0.99","proceeds":"0.7"}}]}"#;
  let (url, seen) = serve(&[("GET /v1/apps/100/appPricePoints", 200, points), ("PATCH /v1/territoryAvailabilities/t2", 200, "{}")]);
  let client = client(&url);
  let found = client.price_points("100", "USA").unwrap();
  assert_eq!(found.iter().map(|point| point.id.as_str()).collect::<Vec<_>>(), ["a", "b"]);
  client.set_territory_available("t2", true).unwrap();
  let seen = seen.lock().unwrap();
  let patch: serde_json::Value = serde_json::from_str(&seen[1].body).unwrap();
  assert_eq!(patch["data"]["attributes"]["available"], true);
}

#[test]
fn territories_have_names_and_can_be_found_by_name_or_code() {
  assert_eq!(territory_name("USA"), "United States");
  assert_eq!(territory_name("ZZZ"), "ZZZ");
  assert_eq!(territory_code("türkiye"), Some("TUR"));
  assert_eq!(territory_code("gbr"), Some("GBR"));
  assert_eq!(territory_code("Atlantis"), None);
}
