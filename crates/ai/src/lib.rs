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

  /// The model used until the user chooses another one in Settings.
  pub fn default_model(self) -> &'static str {
    match self {
      Self::OpenAi => "gpt-4.1",
      Self::Anthropic => "claude-sonnet-5-5",
    }
  }
}

#[derive(Debug)]
pub enum Error {
  MissingKey(Provider),
  Network(String),
  Api { status: u16, message: String },
  EmptyAnswer,
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
    if request.api_key.trim().is_empty() {
      return Err(Error::MissingKey(request.provider));
    }
    let (url, body) = match request.provider {
      Provider::OpenAi => (
        format!("{}/v1/chat/completions", self.openai_url),
        json!({
          "model": request.model,
          "messages": [{"role": "system", "content": request.system}, {"role": "user", "content": request.user}],
          "response_format": {"type": "json_object"},
        }),
      ),
      Provider::Anthropic => (
        format!("{}/v1/messages", self.anthropic_url),
        json!({
          "model": request.model,
          "max_tokens": MAX_OUTPUT_TOKENS,
          "system": request.system,
          "messages": [{"role": "user", "content": request.user}],
        }),
      ),
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
    Ok(answer)
  }
}
