//! App Store review readiness: built-in rules over what App Store Connect tells us, the community playbook
//! (<https://github.com/mjmirza/app-store-compliance>) fetched and cached on this Mac, and an AI analysis that
//! reads both.

mod ai_review;
mod playbook;
mod report;
mod rules;

pub use ai_review::{analyze_with_ai, describe, parse_ai_answer, AiReview};
pub use playbook::{Playbook, PLAYBOOK_REPOSITORY};
pub use report::{Category, Finding, Report, Severity, Verdict};
pub use rules::check;
