use appforge_ai::{Client, Error, Provider, Request};
use std::{
  io::{Read, Write},
  net::TcpListener,
  sync::mpsc,
  thread,
};

/// Serves one canned response and reports the raw request it received.
fn serve_once(status: &str, body: &str) -> (String, mpsc::Receiver<String>) {
  let listener = TcpListener::bind("127.0.0.1:0").unwrap();
  let url = format!("http://{}", listener.local_addr().unwrap());
  let (sender, receiver) = mpsc::channel();
  let response = format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
  thread::spawn(move || {
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
    sender.send(String::from_utf8_lossy(&received).to_string()).unwrap();
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
