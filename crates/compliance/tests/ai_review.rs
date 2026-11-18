use appforge_ai::{Client, Provider};
use appforge_appstore::{App, AppSnapshot, ReviewDetail};
use appforge_compliance::{analyze_with_ai, describe, parse_ai_answer, Category, Playbook, Report, Severity};
use appforge_storage::PrivacyDeclaration;
use std::{
  io::{Read, Write},
  net::TcpListener,
  sync::mpsc,
  thread,
};

fn snapshot() -> AppSnapshot {
  AppSnapshot {
    app: App { id: "1".into(), name: "Example".into(), primary_locale: "en-US".into(), ..Default::default() },
    review_detail: Some(ReviewDetail {
      contact_email: "ada@acme.dev".into(),
      contact_phone: "+1 555 0100".into(),
      demo_account_required: true,
      demo_account_name: "demo-user".into(),
      demo_account_password: "hunter2-secret".into(),
      notes: "Tap Start".into(),
      ..Default::default()
    }),
    ..Default::default()
  }
}

#[test]
fn the_description_sent_to_the_model_leaves_out_passwords_and_contact_details() {
  let text = describe(&snapshot(), &PrivacyDeclaration::default());
  assert!(!text.contains("hunter2-secret"));
  assert!(!text.contains("demo-user"));
  assert!(!text.contains("ada@acme.dev"));
  assert!(!text.contains("+1 555 0100"));
  assert!(text.contains("demo account provided: true"));
  assert!(text.contains("Tap Start"));
}

#[test]
fn answers_wrapped_in_a_code_fence_are_parsed_and_unknown_values_are_tolerated() {
  let answer = "Here you go:\n```json\n{\"summary\":\" Looks risky \",\"findings\":[\
    {\"severity\":\"CRITICAL\",\"category\":\"app_review\",\"title\":\"No demo account\",\"guideline\":\"2.1\",\"explanation\":\"x\",\"fix\":\"y\"},\
    {\"severity\":\"weird\",\"category\":\"other\",\"title\":\"Odd\"},\
    {\"severity\":\"info\",\"title\":\"  \"}]}\n```";
  let review = parse_ai_answer(answer).unwrap();
  assert_eq!(review.summary, "Looks risky");
  assert_eq!(review.findings.len(), 2, "a finding without a title is dropped");
  assert_eq!(review.findings[0].severity, Severity::Critical);
  assert_eq!(review.findings[0].category, Category::AppReview);
  assert_eq!(review.findings[1].severity, Severity::Info);
  assert_eq!(review.findings[1].category, Category::AppInformation);
}

#[test]
fn an_answer_without_json_is_an_error() {
  assert!(parse_ai_answer("I cannot help with that").is_err());
  assert!(parse_ai_answer("{not json}").is_err());
}

fn serve_once(body: String) -> (String, mpsc::Receiver<String>) {
  let listener = TcpListener::bind("127.0.0.1:0").unwrap();
  let url = format!("http://{}", listener.local_addr().unwrap());
  let (sender, receiver) = mpsc::channel();
  thread::spawn(move || {
    let (mut stream, _) = listener.accept().unwrap();
    let mut received = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
      let count = stream.read(&mut buffer).unwrap();
      received.extend_from_slice(&buffer[..count]);
      let text = String::from_utf8_lossy(&received).to_string();
      if let Some((head, rest)) = text.split_once("\r\n\r\n") {
        let length = head
          .lines()
          .find_map(|line| line.to_lowercase().strip_prefix("content-length:").map(|value| value.trim().parse::<usize>().unwrap()))
          .unwrap_or(0);
        if rest.len() >= length {
          break;
        }
      }
      if count == 0 {
        break;
      }
    }
    let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
    stream.write_all(response.as_bytes()).unwrap();
    sender.send(String::from_utf8_lossy(&received).to_string()).unwrap();
  });
  (url, receiver)
}

#[test]
fn the_analysis_sends_the_playbook_as_data_and_returns_findings() {
  let answer = r#"{"summary":"Needs work","findings":[{"severity":"warning","category":"app_privacy","title":"SDK mismatch","guideline":"5.1.1","explanation":"e","fix":"f"}]}"#;
  let body = serde_json::json!({"content": [{"type": "text", "text": answer}]}).to_string();
  let (url, received) = serve_once(body);
  let playbook = Playbook { documents: vec![("APPLE.md".into(), "RULE: demo accounts must work".into())], fetched_at: Some(1) };
  let client = Client::with_base_urls("http://unused.invalid", &url);
  let review = analyze_with_ai(
    &client,
    Provider::Anthropic,
    "key",
    "model",
    &snapshot(),
    &PrivacyDeclaration::default(),
    &Report::default(),
    &playbook,
    &Default::default(),
  )
  .unwrap();
  assert_eq!(review.summary, "Needs work");
  assert_eq!(review.findings[0].title, "SDK mismatch");
  let raw = received.recv().unwrap();
  let body: serde_json::Value = serde_json::from_str(raw.split_once("\r\n\r\n").unwrap().1).unwrap();
  let user = body["messages"][0]["content"].as_str().unwrap();
  assert!(user.contains("RULE: demo accounts must work"));
  assert!(user.contains("<playbook>"));
  assert!(body["system"].as_str().unwrap().contains("untrusted data"));
  assert!(!raw.contains("hunter2-secret"));
}
