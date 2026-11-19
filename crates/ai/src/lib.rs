//! Chat completions from OpenAI and Claude with an API key. Other providers can be added later.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{fmt, time::Duration};

const OPENAI_URL: &str = "https://api.openai.com";
const ANTHROPIC_URL: &str = "https://api.anthropic.com";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const MAX_OUTPUT_TOKENS: u32 = 4096;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
  OpenAi,
  Anthropic,
}

impl Provider {
  pub const ALL: [Provider; 2] = [Provider::Anthropic, Provider::OpenAi];

  pub fn label(self) -> &'static str {
    match self {
      Self::OpenAi => "OpenAI",
      Self::Anthropic => "Claude",
    }
  }

  /// The models offered in Settings: only the ones that cost little per token, cheapest sensible choice first.
  pub fn models(self) -> &'static [ModelInfo] {
    match self {
      Self::OpenAi => OPENAI_MODELS,
      Self::Anthropic => ANTHROPIC_MODELS,
    }
  }

  /// The model used until the user chooses another one in Settings.
  pub fn default_model(self) -> &'static str {
    self.models()[0].id
  }

  /// What `model` is, if it is one of the models offered.
  pub fn model_info(self, model: &str) -> Option<&'static ModelInfo> {
    self.models().iter().find(|info| info.id == model)
  }
}

/// A model that can be chosen, with its price in US dollars per million tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModelInfo {
  pub id: &'static str,
  pub label: &'static str,
  pub input_per_million: f64,
  pub output_per_million: f64,
}

// Prices are the list prices of the providers (checked 2026-11-18); they can change, so Settings says to check the price page.
const ANTHROPIC_MODELS: &[ModelInfo] = &[
  ModelInfo { id: "claude-haiku-5-5", label: "Claude Haiku 5.5", input_per_million: 0.10, output_per_million: 0.50 },
  ModelInfo { id: "claude-haiku-4-5", label: "Claude Haiku 4.5", input_per_million: 1.00, output_per_million: 5.00 },
];

const OPENAI_MODELS: &[ModelInfo] = &[ModelInfo { id: "gpt-6-luna", label: "GPT-6 Luna", input_per_million: 0.10, output_per_million: 0.50 }];

impl ModelInfo {
  /// The price of `usage` in US dollars.
  pub fn cost(&self, usage: Usage) -> f64 {
    (usage.input_tokens as f64 * self.input_per_million + usage.output_tokens as f64 * self.output_per_million) / 1_000_000.
  }

  /// "$0.10 / $0.50": what one million input and output tokens cost.
  pub fn price_text(&self) -> String {
    format!("${:.2} / ${:.2}", self.input_per_million, self.output_per_million)
  }
}

/// How many tokens a request used.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Usage {
  pub input_tokens: u64,
  pub output_tokens: u64,
}

impl Usage {
  pub fn total(self) -> u64 {
    self.input_tokens + self.output_tokens
  }
}

/// The answer of a model and what it cost in tokens.
#[derive(Clone, Debug)]
pub struct Completion {
  pub text: String,
  pub usage: Usage,
}

#[derive(Debug)]
pub enum Error {
  MissingKey(Provider),
  Network(String),
  Api { status: u16, message: String },
  EmptyAnswer,
}

impl Error {
  pub fn is_temporary_service_error(&self) -> bool {
    matches!(self, Self::Api { status: 500 | 502 | 503 | 504 | 529, .. })
  }
}

impl fmt::Display for Error {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::MissingKey(provider) => write!(formatter, "Add your {} API key in Settings first.", provider.label()),
      Self::Network(message) => write!(formatter, "Could not reach the AI provider: {message}"),
      Self::Api { status: 401, .. } => write!(formatter, "The AI provider rejected the API key (401). Check it in Settings."),
      Self::Api { status, message } => write!(formatter, "The AI provider returned an error ({status}): {message}"),
      Self::EmptyAnswer => write!(formatter, "The AI provider returned an empty answer."),
    }
  }
}

impl std::error::Error for Error {}

pub struct Request<'a> {
  pub provider: Provider,
  pub api_key: &'a str,
  pub model: &'a str,
  pub system: &'a str,
  pub user: &'a str,
}

pub struct Client {
  agent: ureq::Agent,
  openai_url: String,
  anthropic_url: String,
}

impl Default for Client {
  fn default() -> Self {
    Self::with_base_urls(OPENAI_URL, ANTHROPIC_URL)
  }
}

impl Client {
  pub fn with_base_urls(openai_url: &str, anthropic_url: &str) -> Self {
    let agent: ureq::Agent = ureq::Agent::config_builder().http_status_as_error(false).timeout_global(Some(Duration::from_secs(180))).build().into();
    Self { agent, openai_url: openai_url.trim_end_matches('/').to_owned(), anthropic_url: anthropic_url.trim_end_matches('/').to_owned() }
  }

  /// Sends one system and one user message and returns the model's text. Blocking: call it off the interface thread.
  pub fn complete(&self, request: &Request) -> Result<String, Error> {
    self.complete_with_usage(request).map(|completion| completion.text)
  }

  /// Retries one explicit temporary service failure, using the same request.
  pub fn complete_with_service_retry(&self, request: &Request) -> Result<Completion, Error> {
    match self.complete_with_usage(request) {
      Err(error) if error.is_temporary_service_error() => {
        std::thread::sleep(Duration::from_secs(1));
        self.complete_with_usage(request)
      }
      result => result,
    }
  }

  /// Like `complete`, and also says how many tokens the request used.
  pub fn complete_with_usage(&self, request: &Request) -> Result<Completion, Error> {
    if request.api_key.trim().is_empty() {
      return Err(Error::MissingKey(request.provider));
    }
    let (url, body) = match request.provider {
      Provider::OpenAi => (format!("{}/v1/chat/completions", self.openai_url), {
        let mut body = json!({
          "model": request.model,
          "messages": [{"role": "system", "content": request.system}, {"role": "user", "content": request.user}],
          "response_format": {"type": "json_object"},
        });
        // GPT-5 models reason before they answer, and reasoning is billed as output: keep it to the least.
        if request.model == "gpt-6-luna" {
          body["reasoning_effort"] = json!("none");
        } else if request.model.starts_with("gpt-5") {
          body["reasoning_effort"] = json!("minimal");
        }
        body
      }),
      Provider::Anthropic => (format!("{}/v1/messages", self.anthropic_url), {
        let mut body = json!({
          "model": request.model,
          "max_tokens": MAX_OUTPUT_TOKENS,
          "system": request.system,
          "messages": [{"role": "user", "content": request.user}],
        });
        // Claude Haiku 5.5 thinks by default, and thinking is billed as output: ask for the least.
        if request.model.starts_with("claude-haiku-5") {
          body["output_config"] = json!({"effort": "low"});
        }
        body
      }),
    };
    let builder = self.agent.post(&url);
    let builder = match request.provider {
      Provider::OpenAi => builder.header("Authorization", &format!("Bearer {}", request.api_key.trim())),
      Provider::Anthropic => builder.header("x-api-key", request.api_key.trim()).header("anthropic-version", ANTHROPIC_VERSION),
    };
    let mut response = builder.send_json(&body).map_err(|error| Error::Network(error.to_string()))?;
    let status = response.status().as_u16();
    let text = response.body_mut().read_to_string().map_err(|error| Error::Network(error.to_string()))?;
    let value: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    if !(200..300).contains(&status) {
      let message = value["error"]["message"].as_str().unwrap_or("No details").to_owned();
      return Err(Error::Api { status, message });
    }
    let answer = match request.provider {
      Provider::OpenAi => value["choices"][0]["message"]["content"].as_str().unwrap_or_default().to_owned(),
      Provider::Anthropic => value["content"]
        .as_array()
        .map(|blocks| blocks.iter().filter_map(|block| block["text"].as_str()).collect::<Vec<_>>().join(""))
        .unwrap_or_default(),
    };
    if answer.trim().is_empty() {
      return Err(Error::EmptyAnswer);
    }
    let (input_key, output_key) = match request.provider {
      Provider::OpenAi => ("prompt_tokens", "completion_tokens"),
      Provider::Anthropic => ("input_tokens", "output_tokens"),
    };
    let usage =
      Usage { input_tokens: value["usage"][input_key].as_u64().unwrap_or(0), output_tokens: value["usage"][output_key].as_u64().unwrap_or(0) };
    Ok(Completion { text: answer, usage })
  }
}
