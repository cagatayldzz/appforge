use appforge_ai::{Provider, Usage};
use appforge_storage::UsageLedger;

fn temporary_dir(name: &str) -> std::path::PathBuf {
  let dir = std::env::temp_dir().join(format!("appforge-{name}-{}", std::process::id()));
  let _ = std::fs::remove_dir_all(&dir);
  std::fs::create_dir_all(&dir).unwrap();
  dir
}

#[test]
fn requests_add_up_per_model_with_a_cost_and_survive_a_restart() {
  let dir = temporary_dir("usage-total");
  let mut ledger = UsageLedger::load(&dir).unwrap();
  assert_eq!(ledger.total().requests, 0);
  let model = Provider::Anthropic.default_model();
  ledger.record(Provider::Anthropic, model, Usage { input_tokens: 1_000_000, output_tokens: 1_000_000 }, 1000);
  ledger.record(Provider::Anthropic, model, Usage { input_tokens: 10, output_tokens: 5 }, 2000);
  ledger.record(Provider::OpenAi, Provider::OpenAi.default_model(), Usage { input_tokens: 100, output_tokens: 0 }, 3000);
  ledger.save(&dir).unwrap();

  let loaded = UsageLedger::load(&dir).unwrap();
  assert_eq!(loaded.since, 1000);
  assert_eq!(loaded.models.len(), 2);
  let total = loaded.total();
  assert_eq!((total.requests, total.input_tokens, total.output_tokens), (3, 1_000_110, 1_000_005));
  assert!(total.cost > 0.59 && total.cost < 0.61);
}

#[test]
fn resetting_clears_the_totals_and_the_start_date() {
  let dir = temporary_dir("usage-reset");
  let mut ledger = UsageLedger::default();
  ledger.record(Provider::OpenAi, "unknown-model", Usage { input_tokens: 5, output_tokens: 5 }, 10);
  assert_eq!(ledger.total().cost, 0.);
  ledger.reset();
  ledger.save(&dir).unwrap();
  let loaded = UsageLedger::load(&dir).unwrap();
  assert_eq!((loaded.since, loaded.total().requests), (0, 0));
}

#[test]
fn an_unreadable_file_is_an_error_not_an_empty_ledger() {
  let dir = temporary_dir("usage-broken");
  std::fs::write(UsageLedger::path(&dir), "not json").unwrap();
  assert!(UsageLedger::load(&dir).is_err());
}
