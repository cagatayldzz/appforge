use appforge_appstore::{App, Client, Credentials, Error, ReviewDetail, VersionLocalization};
use std::{
  collections::HashMap,
  io::{Read, Write},
  net::TcpListener,
  sync::{Arc, Mutex},
  thread,
};

const KEY: &str = include_str!("test_key.p8");

#[derive(Clone, Debug)]
struct Seen {
  method: String,
  path: String,
  authorization: String,
  body: String,
}

/// Serves canned answers by `METHOD /path` (query ignored) and records every request.
fn serve(routes: &[(&str, u16, &str)]) -> (String, Arc<Mutex<Vec<Seen>>>) {
  let listener = TcpListener::bind("127.0.0.1:0").unwrap();
  let url = format!("http://{}", listener.local_addr().unwrap());
  let routes: HashMap<String, (u16, String)> =
    routes.iter().map(|(key, status, body)| (key.to_string(), (*status, body.replace("{base}", &url)))).collect();
  let seen = Arc::new(Mutex::new(Vec::new()));
  let log = seen.clone();
  thread::spawn(move || {
    for stream in listener.incoming() {
      let Ok(mut stream) = stream else { return };
      let routes = routes.clone();
      let log = log.clone();
      thread::spawn(move || {
        let mut received = Vec::new();
        let mut buffer = [0u8; 4096];
        let (head, body) = loop {
          let count = stream.read(&mut buffer).unwrap_or(0);
          received.extend_from_slice(&buffer[..count]);
          let text = String::from_utf8_lossy(&received).to_string();
          if let Some((head, body)) = text.split_once("\r\n\r\n") {
            let length = head
              .lines()
              .find_map(|line| line.to_lowercase().strip_prefix("content-length:").map(|value| value.trim().parse::<usize>().unwrap()))
              .unwrap_or(0);
            if body.len() >= length {
              break (head.to_owned(), body.to_owned());
            }
          }
          if count == 0 {
            return;
          }
        };
        let mut first = head.lines().next().unwrap().split(' ');
        let method = first.next().unwrap().to_owned();
        let target = first.next().unwrap().to_owned();
        let path = target.split('?').next().unwrap().to_owned();
        let authorization = head
          .lines()
          .find_map(|line| line.strip_prefix("authorization: ").or_else(|| line.strip_prefix("Authorization: ")))
          .unwrap_or_default()
          .to_owned();
        log.lock().unwrap().push(Seen { method: method.clone(), path: path.clone(), authorization, body });
        // A route with the exact query wins over one for the bare path.
        let (status, body) = routes
          .get(&format!("{method} {target}"))
          .or_else(|| routes.get(&format!("{method} {path}")))
          .cloned()
          .unwrap_or((404, r#"{"errors":[{"detail":"no route"}]}"#.into()));
        let response =
          format!("HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
        let _ = stream.write_all(response.as_bytes());
      });
    }
  });
  (url, seen)
}

fn client(url: &str) -> Client {
  Client::with_base_url(Credentials { issuer_id: "issuer-1".into(), key_id: "KEYID12345".into(), private_key: KEY.into() }, url)
}

fn app() -> App {
  App {
    id: "100".into(),
    name: "Example".into(),
    bundle_id: "com.example".into(),
    sku: "SKU".into(),
    primary_locale: "en-US".into(),
    ..Default::default()
  }
}

#[test]
fn tokens_are_es256_jwts_with_the_key_id_and_issuer() {
  let credentials = Credentials { issuer_id: " issuer-1 ".into(), key_id: "KEYID12345".into(), private_key: KEY.into() };
  let token = credentials.token(1_700_000_000).unwrap();
  let parts: Vec<&str> = token.split('.').collect();
  assert_eq!(parts.len(), 3);
  let decode = |part: &str| -> serde_json::Value {
    // base64url without padding, decoded with a tiny local decoder to avoid another dependency.
    let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut bits = 0u32;
    let mut count = 0;
    let mut bytes = Vec::new();
    for character in part.bytes() {
      bits = (bits << 6) | alphabet.iter().position(|candidate| *candidate == character).unwrap() as u32;
      count += 6;
      if count >= 8 {
        count -= 8;
        bytes.push((bits >> count) as u8);
        bits &= (1 << count) - 1;
      }
    }
    serde_json::from_slice(&bytes).unwrap()
  };
  let header = decode(parts[0]);
  let claims = decode(parts[1]);
  assert_eq!(header["alg"], "ES256");
  assert_eq!(header["kid"], "KEYID12345");
  assert_eq!(claims["iss"], "issuer-1");
  assert_eq!(claims["aud"], "appstoreconnect-v1");
  assert_eq!(claims["exp"].as_u64().unwrap() - claims["iat"].as_u64().unwrap(), 900);
}

#[test]
fn a_file_that_is_not_a_private_key_is_rejected_before_any_request() {
  assert!(appforge_appstore::validate_private_key(KEY).is_ok());
  assert!(matches!(appforge_appstore::validate_private_key("hello"), Err(Error::InvalidKey(_))));
}

#[test]
fn apps_follow_pagination_and_send_a_bearer_token() {
  let first = r#"{"data":[{"id":"1","attributes":{"name":"One","bundleId":"com.one","sku":"S1","primaryLocale":"en-US"}}],"links":{"next":"{base}/v1/apps?cursor=2"}}"#;
  let second = r#"{"data":[{"id":"2","attributes":{"name":"Two","bundleId":"com.two","sku":"S2","primaryLocale":"tr"}}]}"#;
  let (url, seen) = serve(&[("GET /v1/apps", 200, first), ("GET /v1/apps?cursor=2", 200, second)]);
  let apps = client(&url).apps().unwrap();
  assert_eq!(apps.iter().map(|app| app.name.as_str()).collect::<Vec<_>>(), ["One", "Two"]);
  assert_eq!(apps[0].bundle_id, "com.one");
  let seen = seen.lock().unwrap();
  assert!(seen[0].authorization.starts_with("Bearer ey"));
  assert_eq!(seen.len(), 2, "the next link is followed once");
  assert!(seen.iter().all(|request| request.method == "GET"));
  assert_eq!(seen[1].path, "/v1/apps");
}

#[test]
fn unauthorized_and_forbidden_answers_are_actionable() {
  let (url, _) = serve(&[("GET /v1/apps", 401, r#"{"errors":[{"detail":"bad token"}]}"#)]);
  let text = client(&url).verify().unwrap_err().to_string();
  assert!(text.contains("Issuer ID"));
  let (url, _) = serve(&[("GET /v1/apps", 403, r#"{"errors":[{"detail":"role"}]}"#)]);
  assert!(client(&url).verify().unwrap_err().to_string().contains("role"));
}

#[test]
fn a_snapshot_reads_info_listing_screenshots_and_review_detail() {
  let (url, _) = serve(&[
    (
      "GET /v1/apps/100/appInfos?include=primaryCategory,primarySubcategoryOne,primarySubcategoryTwo,secondaryCategory,secondarySubcategoryOne,secondarySubcategoryTwo,ageRatingDeclaration",
      200,
      r#"{"data":[{"id":"i-live","attributes":{"appStoreState":"READY_FOR_SALE"}},{"id":"i-edit","attributes":{"appStoreState":"PREPARE_FOR_SUBMISSION"}}]}"#,
    ),
    (
      "GET /v1/appInfos/i-edit/territoryAgeRatings?include=territory&limit=200",
      200,
      r#"{"data":[{"attributes":{"appStoreAgeRating":"FOUR_PLUS"},"relationships":{"territory":{"data":{"id":"TUR","type":"territories"}}}}]}"#,
    ),
    (
      "GET /v1/appInfos/i-edit/appInfoLocalizations",
      200,
      r#"{"data":[{"id":"il1","attributes":{"locale":"en-US","name":"Example","subtitle":"Sub","privacyPolicyUrl":"https://example.com/p"}}]}"#,
    ),
    (
      "GET /v1/apps/100/appStoreVersions",
      200,
      r#"{"data":[{"id":"v-live","attributes":{"versionString":"1.0","appStoreState":"READY_FOR_SALE"}},{"id":"v-edit","attributes":{"versionString":"1.1","appVersionState":"PREPARE_FOR_SUBMISSION"}}]}"#,
    ),
    (
      "GET /v1/appStoreVersions/v-edit/appStoreVersionLocalizations",
      200,
      r#"{"data":[{"id":"vl1","attributes":{"locale":"en-US","description":"Desc","keywords":"a,b","supportUrl":"https://example.com/s"}}]}"#,
    ),
    (
      "GET /v1/appStoreVersionLocalizations/vl1/appScreenshotSets",
      200,
      r#"{"data":[{"id":"s1","attributes":{"screenshotDisplayType":"APP_IPHONE_67"},"relationships":{"appScreenshots":{"data":[{"id":"a"},{"id":"b"}]}}}]}"#,
    ),
    (
      "GET /v1/appStoreVersions/v-edit/appStoreReviewDetail",
      200,
      r#"{"data":{"id":"r1","attributes":{"contactFirstName":"Ada","demoAccountRequired":true,"notes":"hi"}}}"#,
    ),
  ]);
  let snapshot = client(&url).snapshot(&app()).unwrap();
  assert_eq!(snapshot.general.territory_ratings, [("TUR".into(), "FOUR_PLUS".into())]);
  assert!(snapshot.general.territory_ratings_error.is_none());
  assert_eq!(snapshot.info_state, "PREPARE_FOR_SUBMISSION");
  assert_eq!(snapshot.info_localizations[0].subtitle, "Sub");
  let version = snapshot.version.unwrap();
  assert_eq!(version.id, "v-edit");
  assert!(version.is_editable());
  assert_eq!(snapshot.version_localizations[0].keywords, "a,b");
  assert_eq!(snapshot.screenshot_sets[0].count, 2);
  let review = snapshot.review_detail.unwrap();
  assert_eq!(review.id.as_deref(), Some("r1"));
  assert!(review.demo_account_required);
}

#[test]
fn a_missing_review_detail_is_none_not_an_error() {
  let (url, _) = serve(&[
    ("GET /v1/apps/100/appInfos", 200, r#"{"data":[]}"#),
    (
      "GET /v1/apps/100/appStoreVersions",
      200,
      r#"{"data":[{"id":"v1","attributes":{"versionString":"1.0","appStoreState":"PREPARE_FOR_SUBMISSION"}}]}"#,
    ),
    ("GET /v1/appStoreVersions/v1/appStoreVersionLocalizations", 200, r#"{"data":[]}"#),
    ("GET /v1/appStoreVersions/v1/appStoreReviewDetail", 200, r#"{"data":null}"#),
  ]);
  let snapshot = client(&url).snapshot(&app()).unwrap();
  assert!(snapshot.review_detail.is_none());
  assert!(snapshot.info_localizations.is_empty());
}

#[test]
fn only_changed_listing_fields_are_patched() {
  let (url, seen) = serve(&[("PATCH /v1/appStoreVersionLocalizations/vl1", 200, r#"{"data":{}}"#)]);
  let original =
    VersionLocalization { id: "vl1".into(), locale: "en-US".into(), description: "Old".into(), keywords: "a".into(), ..Default::default() };
  let mut edited = original.clone();
  assert!(!client(&url).update_version_localization(&edited, &original).unwrap(), "nothing changed, nothing sent");
  edited.description = "New".into();
  assert!(client(&url).update_version_localization(&edited, &original).unwrap());
  let seen = seen.lock().unwrap();
  assert_eq!(seen.len(), 1);
  let body: serde_json::Value = serde_json::from_str(&seen[0].body).unwrap();
  assert_eq!(body["data"]["type"], "appStoreVersionLocalizations");
  assert_eq!(body["data"]["attributes"]["description"], "New");
  assert!(body["data"]["attributes"].get("keywords").is_none());
}

#[test]
fn saving_a_review_detail_creates_it_when_missing_and_patches_it_otherwise() {
  let created = r#"{"data":{"id":"r9","attributes":{"contactFirstName":"Ada","notes":"n"}}}"#;
  let (url, seen) = serve(&[("POST /v1/appStoreReviewDetails", 201, created), ("PATCH /v1/appStoreReviewDetails/r9", 200, created)]);
  let detail = ReviewDetail { contact_first_name: "Ada".into(), notes: "n".into(), ..Default::default() };
  let saved = client(&url).save_review_detail("v1", &detail).unwrap();
  assert_eq!(saved.id.as_deref(), Some("r9"));
  client(&url).save_review_detail("v1", &saved).unwrap();
  let seen = seen.lock().unwrap();
  assert_eq!(seen[0].method, "POST");
  let body: serde_json::Value = serde_json::from_str(&seen[0].body).unwrap();
  assert_eq!(body["data"]["relationships"]["appStoreVersion"]["data"]["id"], "v1");
  assert_eq!(seen[1].method, "PATCH");
}

#[test]
fn the_icon_comes_from_the_latest_build_and_is_downloaded_without_the_token() {
  let builds = r#"{"data":[{"type":"builds","id":"b1","attributes":{"iconAssetToken":{"templateUrl":"{base}/image/{w}x{h}bb.{f}","width":1024,"height":1024}}}]}"#;
  let (url, seen) = serve(&[
    ("GET /v1/builds?filter[app]=100&limit=10&sort=-uploadedDate&fields[builds]=iconAssetToken,expired", 200, builds),
    ("GET /image/128x128bb.png", 200, "ICONBYTES"),
  ]);
  let icon = client(&url).app_icon("100", 128).unwrap();
  assert_eq!(icon, Some(b"ICONBYTES".to_vec()));
  let requests = seen.lock().unwrap();
  assert!(requests[0].authorization.starts_with("Bearer "));
  assert_eq!(requests[1].path, "/image/128x128bb.png");
  assert_eq!(requests[1].authorization, "");
}

#[test]
fn an_app_without_a_build_has_no_icon_and_an_odd_address_is_refused() {
  let (url, _) =
    serve(&[("GET /v1/builds?filter[app]=100&limit=10&sort=-uploadedDate&fields[builds]=iconAssetToken,expired", 200, r#"{"data":[]}"#)]);
  assert_eq!(client(&url).app_icon("100", 128).unwrap(), None);
  let odd = r#"{"data":[{"attributes":{"iconAssetToken":{"templateUrl":"http://example.invalid/{w}x{h}.{f}"}}}]}"#;
  let (url, seen) = serve(&[("GET /v1/builds?filter[app]=100&limit=10&sort=-uploadedDate&fields[builds]=iconAssetToken,expired", 200, odd)]);
  assert!(matches!(client(&url).app_icon("100", 128), Err(Error::Unexpected(_))));
  assert_eq!(seen.lock().unwrap().len(), 1, "the odd address must not be requested");
}

#[test]
fn an_expired_or_icon_less_newest_build_is_skipped_for_an_older_one_with_an_icon() {
  let builds = r#"{"data":[
    {"type":"builds","id":"b3","attributes":{"expired":false}},
    {"type":"builds","id":"b2","attributes":{"expired":true,"iconAssetToken":{"templateUrl":"{base}/image/expired.{f}"}}},
    {"type":"builds","id":"b1","attributes":{"expired":false,"iconAssetToken":{"templateUrl":"{base}/image/{w}x{h}bb.{f}"}}}]}"#;
  let (url, seen) = serve(&[
    ("GET /v1/builds?filter[app]=100&limit=10&sort=-uploadedDate&fields[builds]=iconAssetToken,expired", 200, builds),
    ("GET /image/64x64bb.png", 200, "OLDER"),
  ]);
  assert_eq!(client(&url).app_icon("100", 64).unwrap(), Some(b"OLDER".to_vec()));
  assert_eq!(seen.lock().unwrap().len(), 2);
}

#[test]
fn distribution_commands_keep_draft_submission_and_release_separate() {
  let (url, seen) = serve(&[
    ("POST /v1/appStoreVersions", 201, "{}"),
    ("PATCH /v1/appStoreVersions/version-1/relationships/build", 204, ""),
    ("POST /v1/reviewSubmissions", 201, r#"{"data":{"id":"draft-1"}}"#),
    ("POST /v1/reviewSubmissionItems", 201, "{}"),
    ("PATCH /v1/reviewSubmissions/draft-1", 200, "{}"),
    ("POST /v1/appStoreVersionReleaseRequests", 201, "{}"),
  ]);
  let client = client(&url);
  client.create_ios_version("100", "2.1").unwrap();
  client.select_version_build("version-1", "build-1").unwrap();
  client.add_version_for_review("100", "version-1", None).unwrap();
  assert_eq!(seen.lock().unwrap().len(), 4);
  client.submit_for_review("draft-1").unwrap();
  client.release_version("version-1").unwrap();
  let requests = seen.lock().unwrap();
  let bodies: Vec<serde_json::Value> = requests.iter().map(|request| serde_json::from_str(&request.body).unwrap()).collect();
  assert_eq!(bodies[0]["data"]["attributes"]["releaseType"], "MANUAL");
  assert_eq!(bodies[1]["data"]["id"], "build-1");
  assert_eq!(bodies[3]["data"]["relationships"]["reviewSubmission"]["data"]["id"], "draft-1");
  assert_eq!(bodies[4]["data"]["attributes"]["submitted"], true);
  assert_eq!(bodies[5]["data"]["relationships"]["appStoreVersion"]["data"]["id"], "version-1");
}

#[test]
fn distribution_reuses_a_draft_and_surfaces_a_failed_item() {
  let (url, seen) = serve(&[("POST /v1/reviewSubmissionItems", 409, r#"{"errors":[{"detail":"Build is missing"}]}"#)]);
  let error = client(&url).add_version_for_review("100", "version-1", Some("draft-1")).unwrap_err();
  assert!(matches!(error, Error::Api { status: 409, message } if message == "Build is missing"));
  let requests = seen.lock().unwrap();
  assert_eq!(requests.len(), 1);
  assert_eq!(requests[0].path, "/v1/reviewSubmissionItems");
}

#[test]
fn distribution_finds_a_version_in_a_draft_before_apple_links_it_directly() {
  let (url, _) = serve(&[
    (
      "GET /v1/apps/100/appStoreVersions",
      200,
      r#"{"data":[{"id":"version-1","attributes":{"versionString":"1.0","appVersionState":"PREPARE_FOR_SUBMISSION"}}]}"#,
    ),
    (
      "GET /v1/builds",
      200,
      r#"{"data":[{"id":"build-1","attributes":{"version":"7","processingState":"VALID","buildAudienceType":"APP_STORE_ELIGIBLE"}}]}"#,
    ),
    ("GET /v1/appStoreVersions/version-1/build", 200, r#"{"data":{"id":"build-1"}}"#),
    (
      "GET /v1/apps/100/reviewSubmissions",
      200,
      r#"{"data":[{"id":"draft-1","attributes":{"state":"READY_FOR_REVIEW"},"relationships":{"items":{"data":[{"id":"item-1"}]}}}]}"#,
    ),
    ("GET /v1/reviewSubmissions/draft-1/items", 200, r#"{"data":[{"id":"item-1","relationships":{"appStoreVersion":{"data":{"id":"version-1"}}}}]}"#),
  ]);
  let distribution = client(&url).distribution("100", Some(("version-1", "1.0"))).unwrap();
  assert_eq!(distribution.selected_build.as_deref(), Some("build-1"));
  assert_eq!(distribution.builds[0].number, "7");
  assert!(distribution.phased_release.is_none());
  assert!(distribution.submissions[0].includes_version);
}

#[test]
fn customer_reviews_show_existing_replies_and_write_to_the_correct_resources() {
  let (url, seen) = serve(&[
    (
      "GET /v1/apps/100/customerReviews",
      200,
      r#"{
      "data":[{"id":"review-1","attributes":{"rating":5,"title":"Great"},"relationships":{"response":{"data":{"id":"reply-1"}}}}],
      "included":[{"type":"customerReviewResponses","id":"reply-1","attributes":{"responseBody":"Thanks","state":"PUBLISHED"}}]
    }"#,
    ),
    ("POST /v1/customerReviewResponses", 201, "{}"),
    ("DELETE /v1/customerReviewResponses/reply-1", 204, ""),
  ]);
  let client = client(&url);
  let reviews = client.customer_reviews("100").unwrap();
  assert_eq!(reviews[0].rating, 5);
  assert_eq!(reviews[0].response.as_ref().unwrap().body, "Thanks");
  client.save_customer_review_response("review-1", "Thank you!").unwrap();
  client.delete_customer_review_response("reply-1").unwrap();
  let requests = seen.lock().unwrap();
  let reply: serde_json::Value = serde_json::from_str(&requests[1].body).unwrap();
  assert_eq!(reply["data"]["attributes"]["responseBody"], "Thank you!");
  assert_eq!(reply["data"]["relationships"]["review"]["data"]["id"], "review-1");
  assert_eq!(requests[2].method, "DELETE");
  assert_eq!(requests[2].path, "/v1/customerReviewResponses/reply-1");
}

#[test]
fn product_page_uses_placement_order_and_resolves_image_and_video_previews() {
  let (url, _) = serve(&[(
    "GET /v1/appStoreVersionLocalizations/vl1/placements?include=image,video&sort=placementGroupPosition&limit=50",
    200,
    r#"{"data":[
      {"id":"header","attributes":{"placementType":"PRODUCT_PAGE_HEADER_ASSET","placementGroup":"HEADER"},
       "relationships":{"image":{"data":{"type":"appAssetLibraryImages","id":"im"}}}},
      {"id":"preview","attributes":{"placementType":"APP_PREVIEW","placementGroup":"IPHONE_DYNAMIC_ISLAND_MEDIUM_PROFILE"},
       "relationships":{"video":{"data":{"type":"appAssetLibraryVideos","id":"vi"}}}}
    ],"included":[
      {"type":"appAssetLibraryImages","id":"im","attributes":{"imageAsset":{"templateUrl":"https://images.example/{w}x{h}.{f}","width":3840,"height":1646}}},
      {"type":"appAssetLibraryVideos","id":"vi","attributes":{"videoAsset":"https://media.example/preview.m3u8",
       "previewFrameImage":{"image":{"templateUrl":"https://images.example/poster/{w}x{h}.{f}","width":886,"height":1920}}}}
    ]}"#,
  )]);
  let placements = client(&url).product_assets("vl1").unwrap();
  assert_eq!(placements.iter().map(|placement| placement.id.as_str()).collect::<Vec<_>>(), ["header", "preview"]);
  assert_eq!(placements[0].thumbnail.as_deref(), Some("https://images.example/640x274.jpg"));
  assert_eq!(placements[1].thumbnail.as_deref(), Some("https://images.example/poster/295x640.jpg"));
  assert_eq!(placements[1].video.as_deref(), Some("https://media.example/preview.m3u8"));
}

#[test]
fn retrying_localization_creation_only_adds_the_missing_version_record() {
  let (url, seen) = serve(&[
    ("GET /v1/appInfos/i1/appInfoLocalizations", 200, r#"{"data":[{"attributes":{"locale":"tr"}}]}"#),
    ("GET /v1/appStoreVersions/v1/appStoreVersionLocalizations", 200, r#"{"data":[]}"#),
    ("POST /v1/appStoreVersionLocalizations", 201, r#"{"data":{"id":"tr-version"}}"#),
  ]);
  assert!(client(&url).add_localization("i1", "v1", "tr", "Example", "https://example.com/privacy").unwrap());
  let seen = seen.lock().unwrap();
  let post = seen.iter().filter(|call| call.method == "POST").collect::<Vec<_>>();
  assert_eq!(post.len(), 1);
  let body: serde_json::Value = serde_json::from_str(&post[0].body).unwrap();
  assert_eq!(body["data"]["attributes"]["locale"], "tr");
  assert_eq!(body["data"]["relationships"]["appStoreVersion"]["data"]["id"], "v1");
}

#[test]
fn a_territory_ratings_error_keeps_the_edited_version_available() {
  let (url, _) = serve(&[
    ("GET /v1/apps/100/appInfos", 200, r#"{"data":[{"id":"i1","attributes":{"appStoreAgeRating":"FOUR_PLUS"}}]}"#),
    ("GET /v1/appInfos/i1/appInfoLocalizations", 200, r#"{"data":[]}"#),
    ("GET /v1/appInfos/i1/territoryAgeRatings", 403, r#"{"errors":[{"detail":"Territory ratings are unavailable."}]}"#),
    ("GET /v1/apps/100/appStoreVersions", 200, r#"{"data":[{"id":"v1","attributes":{"appVersionState":"PREPARE_FOR_SUBMISSION"}}]}"#),
    ("GET /v1/appStoreVersions/v1/appStoreVersionLocalizations", 200, r#"{"data":[]}"#),
  ]);
  let snapshot = client(&url).snapshot(&app()).unwrap();
  assert_eq!(snapshot.version.unwrap().id, "v1");
  assert_eq!(snapshot.general.ratings, [("Global".into(), "FOUR_PLUS".into())]);
  assert!(snapshot.general.territory_ratings_error.unwrap().contains("Territory ratings are unavailable."));
}
