//! Live pipeline streams into a running TUI.
use nu_protocol::{ListStream, PipelineData, Signals, Span, Value};
use nu_utils::time::Instant;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::Duration;

/// How long a builder waits for a stream to finish before treating it as
/// live. Long enough for `ls`, `ps`, and `open`; short enough not to stall
/// a `tail -f`.
pub const COLLECT_BUDGET: Duration = Duration::from_millis(250);
/// Rows a builder collects before giving up and going live, so a fast
/// unbounded producer (`1..`) cannot fill memory.
pub const COLLECT_MAX_ITEMS: usize = 100_000;
/// Rows the reader thread may run ahead of the UI before it blocks.
const CHANNEL_CAPACITY: usize = 8192;

pub enum StreamMsg {
    Value(Value),
    Done,
}

/// If `data` is a live stream, read it on a background thread.
/// Collected values stay with the caller (no thread).
pub fn spawn_reader(data: PipelineData) -> Option<Receiver<StreamMsg>> {
    match data {
        PipelineData::ListStream(stream, _) => {
            let (tx, rx) = mpsc::sync_channel(CHANNEL_CAPACITY);
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
            let (tx, rx) = mpsc::sync_channel(CHANNEL_CAPACITY);
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

/// Block until the producer finishes, `timeout` elapses, or `max_items`
/// arrived. The bool is `true` when the producer finished.
pub fn drain_until_idle(
    rx: &Receiver<StreamMsg>,
    timeout: Duration,
    max_items: usize,
) -> (Vec<Value>, bool) {
    let mut items = Vec::new();
    let deadline = Instant::now() + timeout;
    while items.len() < max_items {
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

/// What a builder does with a stream: collect it when it finishes within
/// the budget, otherwise hand back a live stream that starts with the rows
/// already read.
pub enum Collected {
    Done(Vec<Value>),
    Live(ListStream),
}

/// Read `data` (a list or byte stream) for [`COLLECT_BUDGET`].
pub fn collect_briefly(data: PipelineData, span: Span) -> Option<Collected> {
    let rx = spawn_reader(data)?;
    let (items, done) = drain_until_idle(&rx, COLLECT_BUDGET, COLLECT_MAX_ITEMS);
    if done {
        return Some(Collected::Done(items));
    }
    let rest = rx.into_iter().map_while(|msg| match msg {
        StreamMsg::Value(value) => Some(value),
        StreamMsg::Done => None,
    });
    Some(Collected::Live(ListStream::new(
        items.into_iter().chain(rest),
        span,
        Signals::empty(),
    )))
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
