use appforge_ai::{Client, Error, Provider, Request, Usage};
use std::{
  io::{Read, Write},
  net::TcpListener,
  sync::mpsc,
  thread,
};

/// Serves one canned response and reports the raw request it received.
fn serve_once(status: &str, body: &str) -> (String, mpsc::Receiver<String>) {
  serve_responses(&[(status, body)])
}

fn serve_responses(responses: &[(&str, &str)]) -> (String, mpsc::Receiver<String>) {
  let listener = TcpListener::bind("127.0.0.1:0").unwrap();
  let url = format!("http://{}", listener.local_addr().unwrap());
  let (sender, receiver) = mpsc::channel();
  let responses: Vec<_> = responses
    .iter()
    .map(|(status, body)| {
      format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())
    })
    .collect();
  thread::spawn(move || {
    for response in responses {
      let (mut stream, _) = listener.accept().unwrap();
      let mut received = Vec::new();
      let mut buffer = [0u8; 4096];
      loop {
        let count = stream.read(&mut buffer).unwrap();
        received.extend_from_slice(&buffer[..count]);
        let text = String::from_utf8_lossy(&received).to_string();
        if let Some((head, body)) = text.split_once("\r\n\r\n") {
          let length = head
            .lines()
            .find_map(|line| line.to_lowercase().strip_prefix("content-length:").map(|value| value.trim().parse::<usize>().unwrap()))
            .unwrap_or(0);
          if body.len() >= length {
            break;
          }
        }
        if count == 0 {
          break;
        }
      }
      stream.write_all(response.as_bytes()).unwrap();
      let _ = sender.send(String::from_utf8_lossy(&received).to_string());
    }
  });
  (url, receiver)
}

fn request(provider: Provider, key: &str) -> Request<'_> {
  Request { provider, api_key: key, model: "model-x", system: "be brief", user: "hello" }
}

#[test]
fn openai_sends_a_bearer_key_and_reads_the_first_choice() {
  let (url, received) = serve_once("200 OK", r#"{"choices":[{"message":{"content":"{\"ok\":true}"}}]}"#);
  let client = Client::with_base_urls(&url, "http://unused.invalid");
  assert_eq!(client.complete(&request(Provider::OpenAi, "sk-test")).unwrap(), r#"{"ok":true}"#);
  let raw = received.recv().unwrap();
  assert!(raw.starts_with("POST /v1/chat/completions"));
  assert!(raw.to_lowercase().contains("authorization: bearer sk-test"));
  let body: serde_json::Value = serde_json::from_str(raw.split_once("\r\n\r\n").unwrap().1).unwrap();
  assert_eq!(body["model"], "model-x");
  assert_eq!(body["messages"][1]["content"], "hello");
}

#[test]
fn claude_sends_the_key_header_and_joins_text_blocks() {
  let (url, received) = serve_once("200 OK", r#"{"content":[{"type":"text","text":"a"},{"type":"text","text":"b"}]}"#);
  let client = Client::with_base_urls("http://unused.invalid", &url);
  assert_eq!(client.complete(&request(Provider::Anthropic, "ant-key")).unwrap(), "ab");
  let raw = received.recv().unwrap().to_lowercase();
  assert!(raw.starts_with("post /v1/messages"));
  assert!(raw.contains("x-api-key: ant-key"));
  assert!(raw.contains("anthropic-version: 2023-06-01"));
}

#[test]
fn api_errors_carry_the_provider_message_but_never_the_key() {
  let (url, _received) = serve_once("429 Too Many Requests", r#"{"error":{"message":"slow down"}}"#);
  let client = Client::with_base_urls(&url, &url);
  let error = client.complete(&request(Provider::Anthropic, "secret-key")).unwrap_err();
  assert!(matches!(error, Error::Api { status: 429, .. }));
  let text = error.to_string();
  assert!(text.contains("slow down"));
  assert!(!text.contains("secret-key"));
}

#[test]
fn an_unauthorized_key_gets_an_actionable_message() {
  let (url, _received) = serve_once("401 Unauthorized", r#"{"error":{"message":"bad key"}}"#);
  let client = Client::with_base_urls(&url, &url);
  let text = client.complete(&request(Provider::OpenAi, "wrong")).unwrap_err().to_string();
  assert!(text.contains("Settings"));
}

#[test]
fn a_blank_key_fails_before_any_request() {
  let client = Client::with_base_urls("http://unused.invalid", "http://unused.invalid");
  assert!(matches!(client.complete(&request(Provider::OpenAi, "  ")), Err(Error::MissingKey(Provider::OpenAi))));
}

#[test]
fn the_token_usage_is_read_from_each_provider_and_priced() {
  let (url, _) = serve_once("200 OK", r#"{"choices":[{"message":{"content":"{}"}}],"usage":{"prompt_tokens":3000,"completion_tokens":500}}"#);
  let client = Client::with_base_urls(&url, "http://unused.invalid");
  let done = client.complete_with_usage(&request(Provider::OpenAi, "k")).unwrap();
  assert_eq!(done.usage, Usage { input_tokens: 3000, output_tokens: 500 });
  let (url, _) = serve_once("200 OK", r#"{"content":[{"type":"text","text":"{}"}],"usage":{"input_tokens":2000,"output_tokens":100}}"#);
  let client = Client::with_base_urls("http://unused.invalid", &url);
  assert_eq!(client.complete_with_usage(&request(Provider::Anthropic, "k")).unwrap().usage.total(), 2100);
  // 1M input tokens cost the input price, 1M output tokens the output price.
  let haiku = Provider::Anthropic.model_info("claude-haiku-5-5").unwrap();
  let cost = haiku.cost(Usage { input_tokens: 1_000_000, output_tokens: 1_000_000 });
  assert!((cost - 0.60).abs() < 1e-9, "{cost}");
  assert_eq!(haiku.price_text(), "$0.10 / $0.50");
}

#[test]
fn only_cheap_models_are_offered_and_the_first_is_the_default() {
  for provider in Provider::ALL {
    assert!(!provider.models().is_empty());
    assert_eq!(provider.default_model(), provider.models()[0].id);
    assert!(provider.models().iter().all(|model| model.input_per_million <= 1.0 && model.output_per_million <= 5.0), "{provider:?}");
  }
  assert!(Provider::Anthropic.model_info("claude-sonnet-5-5").is_none());
}

#[test]
fn models_that_think_by_default_are_asked_to_think_as_little_as_possible() {
  let (url, received) = serve_once("200 OK", r#"{"content":[{"type":"text","text":"a"}]}"#);
  let client = Client::with_base_urls("http://unused.invalid", &url);
  client.complete(&Request { model: "claude-haiku-5-5", ..request(Provider::Anthropic, "k") }).unwrap();
  let raw = received.recv().unwrap();
  let body: serde_json::Value = serde_json::from_str(raw.split_once("\r\n\r\n").unwrap().1).unwrap();
  assert_eq!(body["output_config"]["effort"], "low");
  for (model, effort) in [("gpt-5-nano", "minimal"), ("gpt-6-luna", "none")] {
    let (url, received) = serve_once("200 OK", r#"{"choices":[{"message":{"content":"{}"}}]}"#);
    let client = Client::with_base_urls(&url, "http://unused.invalid");
    client.complete(&Request { model, ..request(Provider::OpenAi, "k") }).unwrap();
    let body: serde_json::Value = serde_json::from_str(received.recv().unwrap().split_once("\r\n\r\n").unwrap().1).unwrap();
    assert_eq!(body["reasoning_effort"], effort);
  }
}

#[test]
fn a_temporary_provider_error_retries_the_same_analysis_request_once() {
  let (url, received) = serve_responses(&[
    ("500 Internal Server Error", r#"{"error":{"message":"Try again"}}"#),
    ("200 OK", r#"{"choices":[{"message":{"content":"{}"}}]}"#),
  ]);
  let client = Client::with_base_urls(&url, &url);
  let mut retries = 0;
  let completion = client.complete_with_service_retry(&request(Provider::OpenAi, "sk-test"), || retries += 1).unwrap();
  assert_eq!(retries, 1);
  assert_eq!(completion.text, "{}");
  let first = received.recv().unwrap();
  let second = received.recv().unwrap();
  assert_eq!(first.split_once("\r\n\r\n").unwrap().1, second.split_once("\r\n\r\n").unwrap().1);
  assert!(received.recv().is_err());
}
