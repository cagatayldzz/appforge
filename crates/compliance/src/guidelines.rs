//! The Apple review guidelines catalog: download, parse, cache, per-rule changes.

use html5ever::{parse_document, tendril::TendrilSink};
use markup5ever_rcdom::{Handle, NodeData, RcDom};
use serde::{Deserialize, Serialize};
use std::{
  collections::BTreeSet,
  fs,
  io::Write,
  path::Path,
  time::{Duration, SystemTime, UNIX_EPOCH},
};

pub const GUIDELINES_URL: &str = "https://developer.apple.com/app-store/review/guidelines/";
const CACHE: &str = "apple-guidelines.json";

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Guidelines {
  pub rules: Vec<Guideline>,
  pub checked_at: u64,
  pub changes: GuidelineChanges,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct GuidelineChanges {
  pub added: Vec<String>,
  pub modified: Vec<String>,
  pub removed: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Guideline {
  pub number: String,
  pub title: String,
  pub anchor: String,
  pub topic: String,
  text: String,
}

impl Guideline {
  pub fn url(&self) -> String {
    format!("{GUIDELINES_URL}#{}", self.anchor)
  }
}

impl Guidelines {
  pub fn load(dir: &Path) -> Self {
    fs::read(dir.join(CACHE)).ok().and_then(|data| serde_json::from_slice(&data).ok()).unwrap_or_default()
  }

  pub fn refresh(dir: &Path, previous: &Self) -> Result<Self, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(20))).build().into();
    let html = agent
      .get(GUIDELINES_URL)
      .call()
      .and_then(|mut response| response.body_mut().with_config().limit(2_000_000).read_to_string())
      .map_err(|error| format!("Could not download Apple guidelines: {error}"))?;
    let updated = Self::from_html(&html, previous)?;
    let save = || -> Result<(), Box<dyn std::error::Error>> {
      fs::create_dir_all(dir)?;
      let mut file = tempfile::NamedTempFile::new_in(dir)?;
      serde_json::to_writer(&mut file, &updated)?;
      file.flush()?;
      file.persist(dir.join(CACHE))?;
      Ok(())
    };
    save().map_err(|error| format!("Could not save Apple guidelines: {error}"))?;
    Ok(updated)
  }

  pub fn from_html(html: &str, previous: &Self) -> Result<Self, String> {
    let document = parse_document(RcDom::default(), Default::default()).one(html);
    let mut rules = Vec::new();
    walk(&document.document, false, None, &mut rules);
    let ids: BTreeSet<_> = rules.iter().map(|rule| rule.number.as_str()).collect();
    if rules.len() < 60
      || rules.len() < previous.rules.len() / 2
      || ids.len() != rules.len()
      || !["1.2", "2.1", "3.1.1", "4.2", "5.1.1"].iter().all(|id| ids.contains(id))
    {
      return Err("Apple's guidelines page could not be read completely.".into());
    }
    let mut changes = GuidelineChanges::default();
    if !previous.rules.is_empty() {
      for rule in &rules {
        match previous.rule(&rule.number) {
          None => changes.added.push(rule.number.clone()),
          Some(old) if rule != old => changes.modified.push(rule.number.clone()),
          _ => {}
        }
      }
      changes.removed = previous.rules.iter().filter(|rule| !ids.contains(rule.number.as_str())).map(|rule| rule.number.clone()).collect();
      if changes.added.is_empty() && changes.modified.is_empty() && changes.removed.is_empty() {
        changes = previous.changes.clone();
      }
    }
    let checked_at = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    Ok(Self { rules, changes, checked_at })
  }

  pub fn rule(&self, number: &str) -> Option<&Guideline> {
    let number = number.trim();
    let compact = |id: &str| id.replace(['(', ')'], "");
    let number = compact(number);
    self.rules.iter().find(|rule| compact(&rule.number) == number)
  }

  /// Include directly cited rules before topic matches and cap the reference size.
  pub fn context<'a>(&self, numbers: impl IntoIterator<Item = &'a str>, signals: &str, limit: usize) -> String {
    let signals = signals.to_lowercase();
    let mut selected = Vec::new();
    for rule in numbers.into_iter().flat_map(|number| self.rule(number)) {
      if !selected.contains(&rule) {
        selected.push(rule);
      }
    }
    for (terms, ids) in [
      (&["subscription", "purchase", "storekit"][..], &["3.1.1", "3.1.2"][..]),
      (&["account", "login", "privacy", "tracking", "admob"], &["5.1.1", "5.1.1i", "5.1.1v", "5.1.2"]),
      (&["user-generated", "moderation", "anonymous chat"], &["1.2"]),
    ] {
      if terms.iter().any(|term| signals.contains(term)) {
        for rule in ids.iter().filter_map(|id| self.rule(id)) {
          if !selected.contains(&rule) {
            selected.push(rule);
          }
        }
      }
    }
    let mut output = String::new();
    for rule in selected {
      let entry = format!("{} — {}\n{}\n\n", rule.number, rule.title, rule.text);
      let remaining = limit.saturating_sub(output.chars().count());
      if remaining == 0 {
        break;
      }
      output.extend(entry.chars().take(remaining.saturating_sub(1).min(1599)));
      output.push('\n');
    }
    output
  }
}

fn tag(node: &Handle, expected: &str) -> bool {
  matches!(&node.data, NodeData::Element { name, .. } if name.local.as_ref() == expected)
}

fn id(node: &Handle) -> Option<String> {
  let NodeData::Element { attrs, .. } = &node.data else { return None };
  attrs.borrow().iter().find(|attr| attr.name.local.as_ref() == "id").map(|attr| attr.value.to_string())
}

fn label(node: &Handle, skip_rules: bool) -> String {
  fn append(node: &Handle, output: &mut String, skip_rules: bool) {
    if skip_rules && tag(node, "li") && rule_number(node).is_some() {
      return;
    }
    if let NodeData::Text { contents } = &node.data {
      output.push_str(&contents.borrow());
    }
    for child in node.children.borrow().iter() {
      append(child, output, skip_rules);
    }
    if ["p", "li", "br"].iter().any(|name| tag(node, name)) {
      output.push(' ');
    }
  }
  let mut output = String::new();
  append(node, &mut output, skip_rules);
  output.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn numeric_id(value: &str) -> bool {
  let mut parts = value.split('.');
  matches!(parts.next(), Some("1" | "2" | "3" | "4" | "5"))
    && parts.next().is_some_and(|part| part.starts_with(|c: char| c.is_ascii_digit()))
    && value.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'.')
}

fn rule_number(node: &Handle) -> Option<String> {
  id(node)
    .filter(|value| numeric_id(value))
    .or_else(|| node.children.borrow().iter().filter(|child| tag(child, "span")).filter_map(id).find(|value| numeric_id(value)))
}

fn walk(node: &Handle, in_main: bool, parent: Option<&str>, rules: &mut Vec<Guideline>) {
  let in_main = in_main || tag(node, "main");
  let mut inherited = None;
  if in_main && tag(node, "li") {
    if let Some(number) = rule_number(node) {
      let children = node.children.borrow();
      let heading = children.iter().find(|child| tag(child, "strong")).map(|child| label(child, false)).unwrap_or_default();
      let token = heading.split_whitespace().next().unwrap_or_default().trim_end_matches(':');
      let display_number = if numeric_id(&token.replace(['(', ')'], "")) { token.to_owned() } else { number.clone() };
      let title = if token.is_empty() || token.starts_with('(') {
        heading.trim_end_matches(':').to_owned()
      } else {
        heading.strip_prefix(token).unwrap_or_default().trim().trim_end_matches(':').to_owned()
      };
      let section = match number.as_bytes().first() {
        Some(b'1') => "Safety",
        Some(b'2') => "Performance",
        Some(b'3') => "Business",
        Some(b'4') => "Design",
        _ => "Legal",
      };
      let topic = parent.unwrap_or(section).to_owned();
      let text = children.iter().map(|child| label(child, true)).collect::<Vec<_>>().join(" ").split_whitespace().collect::<Vec<_>>().join(" ");
      inherited = (!title.is_empty()).then(|| title.clone());
      rules.push(Guideline { number: display_number, title, anchor: id(node).unwrap_or(number), topic, text });
    }
  }
  for child in node.children.borrow().iter() {
    walk(child, in_main, inherited.as_deref().or(parent), rules);
  }
}
