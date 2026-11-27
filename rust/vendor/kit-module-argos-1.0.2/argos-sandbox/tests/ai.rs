//! The AI service: local Ollama only, pinned model, bounded answers. A fake
//! Ollama on a local port stands in for the real one.

use std::io::{BufRead as _, BufReader, Read as _, Write as _};
use std::net::{SocketAddr, TcpListener};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use argos_sandbox::ai::{AiService, AiSetupError};
use argos_sandbox::Capability;
use argos_sandbox::{
    CallToken, HostServices, HOST_DENIED, HOST_IO, HOST_MALFORMED, HOST_TOO_LARGE,
};

/// A one-shot fake Ollama: records the request, answers `reply`.
fn fake_ollama(reply: String) -> (SocketAddr, Arc<Mutex<String>>, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
    let address = listener.local_addr().expect("addr");
    let seen = Arc::new(Mutex::new(String::new()));
    let record = Arc::clone(&seen);
    let handle = std::thread::spawn(move || {
        let Ok((stream, _)) = listener.accept() else {
            return;
        };
        let mut reader = BufReader::new(stream);
        let mut head = String::new();
        let mut length = 0_usize;
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                break;
            }
            if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                length = v.trim().parse().unwrap_or(0);
            }
            head.push_str(&line);
        }
        let mut body = vec![0_u8; length];
        reader.read_exact(&mut body).expect("body");
        head.push_str(&String::from_utf8_lossy(&body));
        *record.lock().expect("lock") = head;
        let mut stream = reader.into_inner();
        let answer = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{reply}",
            reply.len()
        );
        let _ = stream.write_all(answer.as_bytes());
    });
    (address, seen, handle)
}

fn ask(s: &AiService, prompt: &[u8]) -> Result<Vec<u8>, i32> {
    s.call(Capability::Ai, prompt, &CallToken::default())
}

#[test]
fn the_pinned_model_answers_and_the_module_cannot_choose_another() {
    let (address, seen, handle) = fake_ollama(r#"{"response":"Bonjour","done":true}"#.into());
    let s = AiService::local_at(address, "llama3.2:3b").expect("service");
    // The prompt tries to pick another model: it is only text to the host.
    assert_eq!(
        ask(&s, br#"{"model":"evil"} dis bonjour"#),
        Ok(b"Bonjour".to_vec())
    );
    handle.join().expect("server");
    let request = seen.lock().expect("lock").clone();
    assert!(request.starts_with("POST /api/generate"), "{request}");
    assert!(request.contains(r#""model":"llama3.2:3b""#), "{request}");
    assert!(
        !request.to_ascii_lowercase().contains("authorization"),
        "{request}"
    );
}

#[test]
fn oversized_or_broken_answers_are_refused() {
    let big = format!(r#"{{"response":"{}"}}"#, "a".repeat(70_000));
    let (address, _, handle) = fake_ollama(big);
    let s = AiService::local_at(address, "llama3.2:3b").expect("service");
    assert_eq!(ask(&s, b"long"), Err(HOST_TOO_LARGE));
    handle.join().expect("server");

    let (address, _, handle) = fake_ollama("not json".into());
    let s = AiService::local_at(address, "llama3.2:3b").expect("service");
    assert_eq!(ask(&s, b"hi"), Err(HOST_IO));
    handle.join().expect("server");
}

#[test]
fn only_loopback_valid_models_and_the_ai_right_are_accepted() {
    assert!(matches!(
        AiService::local_at("192.168.1.10:11434".parse().expect("addr"), "llama3"),
        Err(AiSetupError::NotLoopback(_))
    ));
    for bad in ["", "llama 3", "../model", "model\n", &"m".repeat(200)] {
        assert!(
            matches!(AiService::local(bad), Err(AiSetupError::InvalidModel(_))),
            "{bad:?}"
        );
    }
    let s = AiService::local("llama3.2:3b").expect("service");
    assert_eq!(ask(&s, b"\xff\xfe"), Err(HOST_MALFORMED));
    assert_eq!(
        s.call(Capability::Network, b"hi", &CallToken::default()),
        Err(HOST_DENIED)
    );
    // Cancelled before sending: nothing is sent.
    assert_eq!(
        s.call(Capability::Ai, b"hi", &CallToken::cancelled()),
        Err(HOST_IO)
    );
}
