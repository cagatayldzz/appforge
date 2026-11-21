//! A canned App Store Connect server for the client tests.
#![allow(dead_code)]

use appforge_appstore::{Client, Credentials};
use std::{
  collections::HashMap,
  io::{Read, Write},
  net::TcpListener,
  sync::{Arc, Mutex},
  thread,
};

pub const KEY: &str = include_str!("../test_key.p8");

#[derive(Clone, Debug)]
pub struct Seen {
  pub method: String,
  pub path: String,
  pub authorization: String,
  pub body: String,
}

/// Serves canned answers by `METHOD /path` (query ignored) and records every request.
pub fn serve(routes: &[(&str, u16, &str)]) -> (String, Arc<Mutex<Vec<Seen>>>) {
  let listener = TcpListener::bind("127.0.0.1:0").unwrap();
  let url = format!("http://{}", listener.local_addr().unwrap());
  let routes: HashMap<String, (u16, String)> =
    routes.iter().map(|(key, status, body)| (key.to_string(), (*status, body.replace("{base}", &url)))).collect();
  let seen = Arc::new(Mutex::new(Vec::new()));
  let log = seen.clone();
  thread::spawn(move || {
    for stream in listener.incoming() {
      let Ok(mut stream) = stream else { return };
      let routes = routes.clone();
      let log = log.clone();
      thread::spawn(move || {
        let mut received = Vec::new();
        let mut buffer = [0u8; 4096];
        let (head, body) = loop {
          let count = stream.read(&mut buffer).unwrap_or(0);
          received.extend_from_slice(&buffer[..count]);
          let text = String::from_utf8_lossy(&received).to_string();
          if let Some((head, body)) = text.split_once("\r\n\r\n") {
            let length = head
              .lines()
              .find_map(|line| line.to_lowercase().strip_prefix("content-length:").map(|value| value.trim().parse::<usize>().unwrap()))
              .unwrap_or(0);
            if body.len() >= length {
              break (head.to_owned(), body.to_owned());
            }
          }
          if count == 0 {
            return;
          }
        };
        let mut first = head.lines().next().unwrap().split(' ');
        let method = first.next().unwrap().to_owned();
        let target = first.next().unwrap().to_owned();
        let path = target.split('?').next().unwrap().to_owned();
        let authorization = head
          .lines()
          .find_map(|line| line.strip_prefix("authorization: ").or_else(|| line.strip_prefix("Authorization: ")))
          .unwrap_or_default()
          .to_owned();
        log.lock().unwrap().push(Seen { method: method.clone(), path: path.clone(), authorization, body });
        // A route with the exact query wins over one for the bare path.
        let (status, body) = routes
          .get(&format!("{method} {target}"))
          .or_else(|| routes.get(&format!("{method} {path}")))
          .cloned()
          .unwrap_or((404, r#"{"errors":[{"detail":"no route"}]}"#.into()));
        let response =
          format!("HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
        let _ = stream.write_all(response.as_bytes());
      });
    }
  });
  (url, seen)
}

pub fn client(url: &str) -> Client {
  Client::with_base_url(Credentials { issuer_id: "issuer-1".into(), key_id: "KEYID12345".into(), private_key: KEY.into() }, url)
}
