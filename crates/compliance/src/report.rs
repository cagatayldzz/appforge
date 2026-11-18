use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
  /// Apple will very likely reject, or App Store Connect will not let you submit.
  Critical,
  /// A common cause of rejection or a weak listing.
  Warning,
  /// A recommendation.
  Info,
}

impl Severity {
  pub fn label(self) -> &'static str {
    match self {
      Self::Critical => "Critical",
      Self::Warning => "Warning",
      Self::Info => "Tip",
    }
  }
}

/// The page of the app where the finding is fixed.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum Category {
  AppInformation,
  AppReview,
  AppPrivacy,
}

impl Category {
  pub fn label(self) -> &'static str {
    match self {
      Self::AppInformation => "App Information",
      Self::AppReview => "App Review",
      Self::AppPrivacy => "App Privacy",
    }
  }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Finding {
  /// Stable identifier of the rule, for example `info.keywords-too-long`.
  pub id: String,
  pub severity: Severity,
  pub category: Category,
  pub title: String,
  /// The App Store Review Guideline it relates to, for example "2.1" (empty when there is none).
  pub guideline: String,
  pub detail: String,
  pub fix: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Verdict {
  /// No critical findings and no warnings. Still no guarantee: Apple's reviewers decide.
  LooksReady,
  /// No critical findings, but warnings remain.
  NeedsAttention,
  /// At least one critical finding.
  NotReady,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Report {
  pub findings: Vec<Finding>,
}

impl Report {
  pub fn count(&self, severity: Severity) -> usize {
    self.findings.iter().filter(|finding| finding.severity == severity).count()
  }

  pub fn verdict(&self) -> Verdict {
    if self.count(Severity::Critical) > 0 {
      Verdict::NotReady
    } else if self.count(Severity::Warning) > 0 {
      Verdict::NeedsAttention
    } else {
      Verdict::LooksReady
    }
  }

  /// 0 to 100: each critical finding costs 20 points, each warning 7 and each tip 1.
  pub fn score(&self) -> u32 {
    let cost = self.count(Severity::Critical) * 20 + self.count(Severity::Warning) * 7 + self.count(Severity::Info);
    100u32.saturating_sub(cost as u32)
  }
}
