use appforge_storage::Language;
use appforge_ui::i18n::{tr, trf, TURKISH};
use std::{collections::HashSet, fs};

/// Every `.rs` file under `src`, including the ones in subfolders such as `workspace/`.
fn source_files() -> Vec<std::path::PathBuf> {
  fn walk(dir: &std::path::Path, files: &mut Vec<std::path::PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
      let path = entry.unwrap().path();
      if path.is_dir() {
        walk(&path, files);
      } else if path.extension().is_some_and(|extension| extension == "rs") {
        files.push(path);
      }
    }
  }
  let mut files = Vec::new();
  walk(&std::path::PathBuf::from(format!("{}/src", env!("CARGO_MANIFEST_DIR"))), &mut files);
  files
}

/// Every literal passed to `tr(l, "…")` or `trf(l, "…", …)` in the interface sources.
fn used_texts() -> Vec<String> {
  let mut texts = Vec::new();
  for path in source_files() {
    if path.file_name().is_some_and(|name| name == "i18n.rs") {
      continue;
    }
    let source = fs::read_to_string(&path).unwrap();
    for call in ["tr(", "trf("] {
      for (start, _) in source.match_indices(call) {
        if source[..start].chars().next_back().is_some_and(|c| c.is_alphanumeric() || c == '_') {
          continue;
        }
        let rest = &source[start + call.len()..];
        let Some(comma) = rest.find(',') else { continue };
        let after = rest[comma + 1..].trim_start();
        let Some(body) = after.strip_prefix('"') else { continue };
        let mut text = String::new();
        let mut characters = body.chars();
        while let Some(character) = characters.next() {
          match character {
            '"' => break,
            '\\' => text.push(match characters.next() {
              Some('n') => '\n',
              Some(other) => other,
              None => break,
            }),
            other => text.push(other),
          }
        }
        texts.push(text);
      }
    }
  }
  texts
}

#[test]
fn every_text_used_in_the_interface_has_a_turkish_translation() {
  let known: HashSet<&str> = TURKISH.iter().map(|(english, _)| *english).collect();
  let used = used_texts();
  assert!(used.len() > 100, "the scanner found only {} texts", used.len());
  let missing: Vec<String> = used.into_iter().filter(|text| !known.contains(text.as_str())).collect();
  assert!(missing.is_empty(), "missing Turkish translations: {missing:#?}");
}

#[test]
fn translations_have_no_duplicates_and_keep_their_placeholders() {
  let mut seen = HashSet::new();
  for (english, turkish) in TURKISH {
    assert!(seen.insert(*english), "duplicate entry: {english}");
    assert!(!turkish.is_empty(), "empty translation: {english}");
    assert_eq!(english.matches("{}").count(), turkish.matches("{}").count(), "placeholders differ: {english}");
  }
}

#[test]
fn english_is_the_source_and_unknown_text_falls_back_to_it() {
  assert_eq!(tr(Language::English, "Settings"), "Settings");
  assert_eq!(tr(Language::Turkish, "Settings"), "Ayarlar");
  assert_eq!(tr(Language::Turkish, "No translation for this text"), "No translation for this text");
}

#[test]
fn placeholders_are_filled_in_order() {
  assert_eq!(trf(Language::English, "Open {}", &["Analysis"]), "Open Analysis");
  assert_eq!(trf(Language::Turkish, "{} critical · {} · {}", &["1", "2 uyarı", "3 ipucu"]), "1 kritik · 2 uyarı · 3 ipucu");
}

#[test]
fn no_visible_text_is_left_untranslated_in_the_interface() {
  let mut raw = Vec::new();
  for path in source_files() {
    if path.file_name().is_some_and(|name| name == "i18n.rs" || name == "forms.rs") {
      continue;
    }
    let source = fs::read_to_string(&path).unwrap();
    for call in [".child(\"", ".label(\"", ".tooltip(\"", "numbered(1, \"", "numbered(2, \"", "numbered(3, \"", "numbered(4, \""] {
      for (start, _) in source.match_indices(call) {
        let text: String = source[start + call.len()..].chars().take_while(|c| *c != '"').collect();
        if text.chars().next().is_some_and(|c| c.is_uppercase()) && text != "AppForge" {
          raw.push(format!("{}: {text}", path.display()));
        }
      }
    }
  }
  assert!(raw.is_empty(), "text not passed through tr/trf: {raw:#?}");
}
