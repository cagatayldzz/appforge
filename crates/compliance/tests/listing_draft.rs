use appforge_compliance::{listing_prompt, parse_listing_answer, ListingFields, KEYWORDS_LIMIT, SUBTITLE_LIMIT};

#[test]
fn an_answer_in_a_code_fence_is_read_and_cut_to_apples_limits() {
  let answer = format!(
    "Here you go:\n```json\n{{\"subtitle\": \"{}\", \"description\": \"  A calm word puzzle.  \", \"keywords\": \" word , puzzle,, hexagon \", \"promotional_text\": \"\", \"whats_new\": \"First release.\"}}\n```",
    "x".repeat(80)
  );
  let draft = parse_listing_answer(&answer).unwrap();
  assert_eq!(draft.subtitle.chars().count(), SUBTITLE_LIMIT);
  assert_eq!(draft.description, "A calm word puzzle.");
  assert_eq!(draft.keywords, "word,puzzle,hexagon");
  assert_eq!(draft.promotional_text, "");
  assert_eq!(draft.whats_new, "First release.");
}

#[test]
fn too_many_keywords_lose_whole_words_not_half_words() {
  let words: Vec<String> = (0..40).map(|index| format!("word{index}")).collect();
  let answer = serde_json::json!({"keywords": words.join(", ")}).to_string();
  let keywords = parse_listing_answer(&answer).unwrap().keywords;
  assert!(keywords.chars().count() <= KEYWORDS_LIMIT);
  assert!(keywords.split(',').all(|word| word.starts_with("word") && word.len() >= 5), "{keywords}");
  assert!(!keywords.ends_with(','));
}

#[test]
fn an_answer_without_json_is_an_error() {
  assert!(parse_listing_answer("Sorry, I cannot help with that.").is_err());
  assert!(parse_listing_answer("{not json}").is_err());
}

#[test]
fn the_prompt_has_the_locale_the_current_listing_and_the_project() {
  let current = ListingFields { name: "Hexumi".into(), subtitle: "".into(), description: "Old text".into(), ..Default::default() };
  let prompt = listing_prompt(&current, "tr", "FOLDER STRUCTURE (top levels):\nSources/\n");
  assert!(prompt.contains("LOCALE: tr"));
  assert!(prompt.contains("name: Hexumi"));
  assert!(prompt.contains("subtitle: (empty)"));
  assert!(prompt.contains("Old text"));
  assert!(prompt.contains("Sources/"));
}
