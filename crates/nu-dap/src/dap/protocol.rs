//! DAP base protocol: message framing (Content-Length headers over stdio)
//! and the three message envelopes (request / response / event).
//!
//! Spec: <https://microsoft.github.io/debug-adapter-protocol/specification>

use serde::{Deserialize, Serialize};
use serde_json::Value as Json;
use std::io::{BufRead, Write};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Debug, Deserialize)]
pub struct Request {
    pub seq: i64,
    pub command: String,
    #[serde(default)]
    pub arguments: Json,
}

#[derive(Debug, Serialize)]
pub struct Response {
    pub seq: i64,
    #[serde(rename = "type")]
    pub type_: &'static str, // "response"
    pub request_seq: i64,
    pub success: bool,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Json::is_null")]
    pub body: Json,
}

#[derive(Debug, Serialize)]
pub struct Event {
    pub seq: i64,
    #[serde(rename = "type")]
    pub type_: &'static str, // "event"
    pub event: &'static str,
    #[serde(skip_serializing_if = "Json::is_null")]
    pub body: Json,
}

/// Reads one DAP message from the reader. Returns None on EOF.
pub fn read_message<R: BufRead>(reader: &mut R) -> std::io::Result<Option<Request>> {
    let mut content_length: Option<usize> = None;

    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            return Ok(None); // EOF
        }

        let line = line.trim_end();
        if line.is_empty() {
            break; // end of headers
        }

        if let Some(rest) = line.strip_prefix("Content-Length:") {
            content_length = rest.trim().parse().ok();
        }
    }

    let len = content_length.ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, "missing Content-Length")
    })?;

    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf)?;
    let req: Request = serde_json::from_slice(&buf)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    Ok(Some(req))
}

/// Thread-safe writer for responses and events. Cloned into the eval thread
/// so the Debugger impl can emit `stopped`/`output`/`terminated` events.
#[derive(Clone)]
pub struct DapWriter {
    inner: Arc<Mutex<Box<dyn Write + Send>>>,
    seq: Arc<AtomicI64>,
}

impl DapWriter {
    pub fn new(w: Box<dyn Write + Send>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(w)),
            seq: Arc::new(AtomicI64::new(1)),
        }
    }

    fn next_seq(&self) -> i64 {
        self.seq.fetch_add(1, Ordering::SeqCst)
    }

    fn write_json(&self, json: &impl Serialize) {
        let payload = serde_json::to_vec(json).expect("serialize DAP message");
        let mut w = self.inner.lock().expect("DAP writer poisoned");
        // Ignore write errors: if the client is gone we are shutting down anyway.
        let _ = write!(w, "Content-Length: {}\r\n\r\n", payload.len());
        let _ = w.write_all(&payload);
        let _ = w.flush();
    }

    pub fn respond(&self, req_seq: i64, command: &str, body: Json) {
        self.write_json(&Response {
            seq: self.next_seq(),
            type_: "response",
            request_seq: req_seq,
            success: true,
            command: command.to_string(),
            message: None,
            body,
        });
    }

    pub fn respond_error(&self, req_seq: i64, command: &str, message: impl Into<String>) {
        self.write_json(&Response {
            seq: self.next_seq(),
            type_: "response",
            request_seq: req_seq,
            success: false,
            command: command.to_string(),
            message: Some(message.into()),
            body: Json::Null,
        });
    }

    pub fn event(&self, event: &'static str, body: Json) {
        self.write_json(&Event {
            seq: self.next_seq(),
            type_: "event",
            event,
            body,
        });
    }

    pub fn output(&self, category: &str, text: impl Into<String>) {
        self.event(
            "output",
            serde_json::json!({ "category": category, "output": text.into() }),
        );
    }
}
