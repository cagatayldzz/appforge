//! App Store review readiness: built-in rules over what App Store Connect tells us, the community playbook
//! (<https://github.com/mjmirza/app-store-compliance>) fetched and cached on this Mac, and an AI analysis that
//! reads both.

mod ai_review;
mod listing_draft;
mod playbook;
mod report;
mod rules;

pub use ai_review::{analyze_with_ai, describe, parse_ai_answer, AiReview};
pub use listing_draft::{
  draft_listing, listing_prompt, parse_listing_answer, ListingDraft, ListingFields, DESCRIPTION_LIMIT, KEYWORDS_LIMIT, PROMOTIONAL_LIMIT,
  SUBTITLE_LIMIT, WHATS_NEW_LIMIT,
};
pub use playbook::{Playbook, PLAYBOOK_REPOSITORY};
pub use report::{Category, Finding, Report, Severity, Verdict};
pub use rules::check;
