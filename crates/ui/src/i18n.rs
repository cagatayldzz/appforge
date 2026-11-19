//! Interface translations, kept in `crates/ui/locales/<language>.json` as flat `"key": "text"` objects, for example
//! `"createAccount": "Create Account"`. Code passes the key to `tr`/`trf`; `en.json` holds the English wording. `{}` in a
//! text is a placeholder that `trf` fills in order, and both languages use the same placeholders.
//!
//! English text that comes from another crate (the rule categories and severities of `compliance`) is not a key: `tr` finds
//! it by its English wording in `en.json` and returns the text of the language asked for. Anything unknown is shown as it is.
//!
//! To add a language: add a `locales/xx.json`, a `Language` variant in `storage`, and a line in `source`.

use appforge_storage::Language;
use gpui_kit::SharedString;
use serde::{
  de::{MapAccess, Visitor},
  Deserialize, Deserializer,
};
use std::{collections::HashMap, fmt, sync::OnceLock};

const ENGLISH: &str = include_str!("../locales/en.json");
const TURKISH: &str = include_str!("../locales/tr.json");

/// The entries of a file in file order, duplicates kept, so a test can see them.
struct Entries(Vec<(String, String)>);

impl<'de> Deserialize<'de> for Entries {
  fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
    struct Collect;
    impl<'de> Visitor<'de> for Collect {
      type Value = Entries;

      fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an object of text keys and text values")
      }

      fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Entries, A::Error> {
        let mut entries = Vec::new();
        while let Some(entry) = map.next_entry::<String, String>()? {
          entries.push(entry);
        }
        Ok(Entries(entries))
      }
    }
    deserializer.deserialize_map(Collect)
  }
}

/// One language, read once. The texts live for the whole run, so `tr` hands out `&'static str` without copying.
struct Catalog {
  entries: Vec<(&'static str, &'static str)>,
  by_key: HashMap<&'static str, &'static str>,
  /// The English wording of each key, to the text of this language: for English that comes from another crate.
  from_english: HashMap<&'static str, &'static str>,
}

fn source(language: Language) -> &'static str {
  match language {
    Language::English => ENGLISH,
    Language::Turkish => TURKISH,
  }
}

fn catalog(language: Language) -> &'static Catalog {
  static ENGLISH_CATALOG: OnceLock<Catalog> = OnceLock::new();
  static TURKISH_CATALOG: OnceLock<Catalog> = OnceLock::new();
  let cell = match language {
    Language::English => &ENGLISH_CATALOG,
    Language::Turkish => &TURKISH_CATALOG,
  };
  cell.get_or_init(|| {
    let Entries(parsed) = serde_json::from_str(source(language)).expect("the locale file is valid JSON of text keys and values");
    let leak = |text: String| -> &'static str { Box::leak(text.into_boxed_str()) };
    let entries: Vec<(&'static str, &'static str)> = parsed.into_iter().map(|(key, text)| (leak(key), leak(text))).collect();
    let by_key: HashMap<&'static str, &'static str> = entries.iter().copied().collect();
    let from_english = match language {
      Language::English => entries.iter().map(|(_, text)| (*text, *text)).collect(),
      _ => catalog(Language::English).entries.iter().filter_map(|(key, text)| by_key.get(key).map(|translated| (*text, *translated))).collect(),
    };
    Catalog { entries, by_key, from_english }
  })
}

/// Every `(key, text)` of a language in file order, duplicate keys included. For tests and tools.
pub fn entries(language: Language) -> &'static [(&'static str, &'static str)] {
  &catalog(language).entries
}

/// The text of `key` in `language`. Unknown text is shown as it is.
pub fn tr(language: Language, key: &'static str) -> SharedString {
  let catalog = catalog(language);
  SharedString::new_static(catalog.by_key.get(key).or_else(|| catalog.from_english.get(key)).copied().unwrap_or(key))
}

pub(crate) fn translated_key(language: Language, key: &str) -> SharedString {
  catalog(language).by_key.get(key).map_or_else(|| SharedString::from(key.to_owned()), |value| SharedString::new_static(value))
}

pub(crate) fn translated_text(language: Language, text: &str) -> SharedString {
  catalog(language).from_english.get(text).map_or_else(|| SharedString::from(text.to_owned()), |value| SharedString::new_static(value))
}

/// `text` in `language`, with each `{}` replaced by the next of `args`.
pub fn trf(language: Language, key: &'static str, args: &[&str]) -> SharedString {
  let template = tr(language, key);
  let mut parts = template.split("{}");
  let mut out = String::from(parts.next().unwrap_or_default());
  for (part, arg) in parts.zip(args.iter().copied().chain(std::iter::repeat(""))) {
    out.push_str(arg);
    out.push_str(part);
  }
  out.into()
}
