use appforge_compliance::Playbook;
use std::{
  fs,
  io::{Read, Write},
  net::TcpListener,
  sync::atomic::{AtomicUsize, Ordering},
  thread,
};

fn temporary_dir() -> std::path::PathBuf {
  // The counter keeps parallel tests apart: the clock alone can return the same value twice.
  static NEXT: AtomicUsize = AtomicUsize::new(0);
  let dir = std::env::temp_dir().join(format!("appforge-playbook-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
  fs::create_dir_all(&dir).unwrap();
  dir
}

/// Serves `/APPLE.md`, `/PRE-SUBMISSION-CHECKLIST.md` and (unless `missing`) `/MISTAKE-PATTERNS.md`.
fn serve(missing: Option<&'static str>) -> String {
  let listener = TcpListener::bind("127.0.0.1:0").unwrap();
  let url = format!("http://{}", listener.local_addr().unwrap());
  thread::spawn(move || {
    for stream in listener.incoming() {
      let Ok(mut stream) = stream else { return };
      let mut buffer = [0u8; 4096];
      let count = stream.read(&mut buffer).unwrap_or(0);
      let request = String::from_utf8_lossy(&buffer[..count]).to_string();
      let path = request.split(' ').nth(1).unwrap_or("/").to_owned();
      let response = if missing.is_some_and(|name| path.ends_with(name)) {
        "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_owned()
      } else {
        let body = format!("# {path}\nrules");
        format!("HTTP/1.1 200 OK\r\nContent-Type: text/markdown\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())
      };
      let _ = stream.write_all(response.as_bytes());
    }
  });
  url
}

#[test]
fn refreshing_downloads_the_pages_and_the_cache_survives_a_restart() {
  let dir = temporary_dir();
  assert!(Playbook::load(&dir).is_empty());
  let playbook = Playbook::refresh_from(&dir, &serve(None)).unwrap();
  assert_eq!(playbook.documents.len(), 3);
  assert!(playbook.fetched_at.is_some());
  let cached = Playbook::load(&dir);
  assert_eq!(cached, playbook);
  assert!(cached.text(1_000_000).contains("### APPLE.md"));
  fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_failed_download_keeps_the_previous_cache() {
  let dir = temporary_dir();
  Playbook::refresh_from(&dir, &serve(None)).unwrap();
  let before = Playbook::load(&dir);
  let error = Playbook::refresh_from(&dir, &serve(Some("MISTAKE-PATTERNS.md"))).unwrap_err();
  assert!(error.contains("MISTAKE-PATTERNS.md"));
  assert_eq!(Playbook::load(&dir), before);
  fs::remove_dir_all(dir).unwrap();
}

#[test]
fn the_text_is_cut_to_the_limit() {
  let playbook = Playbook { documents: vec![("A.md".into(), "x".repeat(500)), ("B.md".into(), "y".repeat(500))], fetched_at: None };
  let text = playbook.text(100);
  assert!(text.contains("### A.md"));
  assert!(!text.contains("### B.md"));
  assert!(text.matches('x').count() <= 100);
}
