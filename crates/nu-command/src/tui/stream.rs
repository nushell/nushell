//! Live pipeline streams into a running TUI.
use nu_protocol::{PipelineData, Value};
use nu_utils::time::Instant;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::Duration;

pub enum StreamMsg {
    Value(Value),
    Done,
}

/// If `data` is a live stream, read it on a background thread.
/// Collected values stay with the caller (no thread).
pub fn spawn_reader(data: PipelineData) -> Option<Receiver<StreamMsg>> {
    match data {
        PipelineData::ListStream(stream, _) => {
            let (tx, rx) = mpsc::channel();
            thread::Builder::new()
                .name("nu-tui-stream".into())
                .spawn(move || {
                    for value in stream {
                        if tx.send(StreamMsg::Value(value)).is_err() {
                            return;
                        }
                    }
                    let _ = tx.send(StreamMsg::Done);
                })
                .ok()?;
            Some(rx)
        }
        PipelineData::ByteStream(stream, _) => {
            let span = stream.span();
            let lines = stream.lines()?;
            let (tx, rx) = mpsc::channel();
            thread::Builder::new()
                .name("nu-tui-bytes".into())
                .spawn(move || {
                    for line in lines {
                        match line {
                            Ok(text) => {
                                if tx
                                    .send(StreamMsg::Value(Value::string(text, span)))
                                    .is_err()
                                {
                                    return;
                                }
                            }
                            Err(_) => break,
                        }
                    }
                    let _ = tx.send(StreamMsg::Done);
                })
                .ok()?;
            Some(rx)
        }
        _ => None,
    }
}

pub fn drain_available(rx: &Receiver<StreamMsg>) -> (Vec<Value>, bool) {
    let mut items = Vec::new();
    let mut done = false;
    loop {
        match rx.try_recv() {
            Ok(StreamMsg::Value(value)) => items.push(value),
            Ok(StreamMsg::Done) => {
                done = true;
                break;
            }
            Err(TryRecvError::Empty) => break,
            Err(TryRecvError::Disconnected) => {
                done = true;
                break;
            }
        }
    }
    (items, done)
}

/// Block until the producer finishes or `timeout` elapses (headless tests).
pub fn drain_until_idle(rx: &Receiver<StreamMsg>, timeout: Duration) -> (Vec<Value>, bool) {
    let mut items = Vec::new();
    let deadline = Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        match rx.recv_timeout(remaining) {
            Ok(StreamMsg::Value(value)) => items.push(value),
            Ok(StreamMsg::Done) => return (items, true),
            Err(_) => break,
        }
    }
    (items, false)
}

pub fn collected_value(data: PipelineData) -> Option<Value> {
    match data {
        PipelineData::Value(value, _) if !matches!(value, Value::Nothing { .. }) => Some(value),
        PipelineData::Empty => None,
        PipelineData::ListStream(stream, _) => stream.into_value().ok(),
        PipelineData::ByteStream(stream, _) => stream.into_value().ok(),
        PipelineData::Value(_, _) => None,
    }
}
