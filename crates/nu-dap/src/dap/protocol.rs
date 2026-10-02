//! DAP base protocol: message framing (Content-Length headers over stdio)
//! and the three message envelopes (request / response / event).
//!
//! Spec: <https://microsoft.github.io/debug-adapter-protocol/specification>

use crate::dap::types::{DapEvent, ResponseBody};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;
use std::io::{BufRead, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

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

/// `event` and `body` come from the flattened [`DapEvent`]: adjacent tagging
/// writes the variant's name into `event` and its fields into `body`.
#[derive(Debug, Serialize)]
pub struct Event {
    pub seq: i64,
    #[serde(rename = "type")]
    pub type_: &'static str, // "event"
    #[serde(flatten)]
    pub event: DapEvent,
}

/// Largest DAP message body we will allocate for. Real requests are a few KB
/// at most; the cap is what stops a malformed or hostile `Content-Length` from
/// turning into a multi-gigabyte allocation (and an OOM abort) before a single
/// byte of the body has arrived.
const MAX_CONTENT_LENGTH: usize = 64 * 1024 * 1024;

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

        // Header names are case-insensitive, as in HTTP.
        if let Some((name, value)) = line.split_once(':')
            && name.trim().eq_ignore_ascii_case("Content-Length")
        {
            content_length = value.trim().parse().ok();
        }
    }

    let len = content_length.ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, "missing Content-Length")
    })?;

    if len > MAX_CONTENT_LENGTH {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Content-Length {len} exceeds the {MAX_CONTENT_LENGTH} byte limit"),
        ));
    }

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
        let mut w = self.inner.lock();
        // Ignore write errors: if the client is gone we are shutting down anyway.
        let _ = write!(w, "Content-Length: {}\r\n\r\n", payload.len());
        let _ = w.write_all(&payload);
        let _ = w.flush();
    }

    pub fn respond(&self, req_seq: i64, command: &str, body: impl ResponseBody) {
        self.write_json(&Response {
            seq: self.next_seq(),
            type_: "response",
            request_seq: req_seq,
            success: true,
            command: command.to_string(),
            message: None,
            body: to_body(&body),
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

    pub fn event(&self, event: DapEvent) {
        self.write_json(&Event {
            seq: self.next_seq(),
            type_: "event",
            event,
        });
    }

    pub fn output(&self, category: &str, text: impl Into<String>) {
        self.event(DapEvent::Output {
            category: category.to_string(),
            output: text.into(),
        });
    }
}

/// Bodies are plain owned structs, so serialization cannot fail; a bodyless
/// response serializes to null and the envelope skips it.
fn to_body(body: &impl Serialize) -> Json {
    serde_json::to_value(body).expect("serialize DAP body")
}

#[cfg(test)]
mod tests {
    //! Unit tests for [`crate::dap::protocol`].

    use super::{MAX_CONTENT_LENGTH, read_message};
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    /// One framed message, ready for [`read_message`].
    fn framed(header: &str, body: &str) -> std::io::Cursor<Vec<u8>> {
        std::io::Cursor::new(format!("{header}: {}\r\n\r\n{body}", body.len()).into_bytes())
    }

    #[rstest]
    #[case::canonical("Content-Length")]
    #[case::lowercase("content-length")]
    #[case::shouting("CONTENT-LENGTH")]
    fn header_name_is_case_insensitive(#[case] header: &str) {
        let mut reader = framed(header, r#"{"seq":1,"command":"initialize"}"#);
        let req = read_message(&mut reader)
            .expect("reads")
            .expect("not at EOF");
        assert_eq!(req.seq, 1);
        assert_eq!(req.command, "initialize");
    }

    #[test]
    fn unrelated_headers_are_ignored() {
        let body = r#"{"seq":2,"command":"threads"}"#;
        let raw = format!(
            "Content-Type: application/vscode-jsonrpc\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        let mut reader = std::io::Cursor::new(raw.into_bytes());
        let req = read_message(&mut reader)
            .expect("reads")
            .expect("not at EOF");
        assert_eq!(req.command, "threads");
    }

    #[test]
    fn eof_before_any_header_is_not_an_error() {
        let mut reader = std::io::Cursor::new(Vec::new());
        assert!(read_message(&mut reader).expect("reads").is_none());
    }

    #[test]
    fn a_missing_length_ends_the_session() {
        let mut reader = std::io::Cursor::new(b"Content-Type: nonsense\r\n\r\n".to_vec());
        let err = read_message(&mut reader).expect_err("no length to read a body by");
        assert!(err.to_string().contains("missing Content-Length"));
    }

    #[test]
    fn an_oversized_length_is_refused_without_allocating() {
        let raw = format!("Content-Length: {}\r\n\r\n", MAX_CONTENT_LENGTH + 1);
        let mut reader = std::io::Cursor::new(raw.into_bytes());
        let err = read_message(&mut reader).expect_err("over the cap");
        assert!(err.to_string().contains("exceeds"));
    }
}
