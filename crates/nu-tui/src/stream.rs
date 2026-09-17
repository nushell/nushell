//! Live pipeline streams into a running TUI.
use nu_protocol::{ByteStreamSource, ListStream, PipelineData, Signals, Span, Value};
use nu_utils::time::Instant;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::Duration;

/// Rows a builder collects before handing the rest on as a live stream, so
/// a fast unbounded producer (`1.. | each { ... }`) cannot fill memory.
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
        PipelineData::ListStream(stream, _) => Some(spawn_values(stream.into_iter())),
        PipelineData::ByteStream(stream, _) => {
            let span = stream.span();
            // A child without a stdout pipe has nothing to read: an empty
            // stream, not an error.
            let lines = stream.lines()?;
            let lines = lines
                .map_while(|line| line.ok())
                .map(move |text| Value::string(text, span));
            Some(spawn_values(lines))
        }
        _ => None,
    }
}

fn spawn_values(values: impl Iterator<Item = Value> + Send + 'static) -> Receiver<StreamMsg> {
    let (tx, rx) = mpsc::sync_channel(CHANNEL_CAPACITY);
    // The thread ends when the producer does or when the receiver is gone.
    // A producer blocked inside `next()` (an external command that never
    // writes) cannot be interrupted; `Collected::Live` carries its pid so
    // the TUI can kill it instead.
    let _ = thread::Builder::new()
        .name("nu-tui-stream".into())
        .spawn(move || {
            for value in values {
                if tx.send(StreamMsg::Value(value)).is_err() {
                    return;
                }
            }
            let _ = tx.send(StreamMsg::Done);
        });
    rx
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

/// What a builder does with a stream: collect it, or hand back a live
/// stream that starts with the rows already read.
pub enum Collected {
    Done(Vec<Value>),
    Live {
        stream: ListStream,
        /// The external command producing the stream, to kill when the TUI
        /// closes before it ends.
        child_pid: Option<u32>,
    },
}

/// Read a list or byte stream for a builder. The decision is made by the
/// stream's kind, never by timing, so `[(ls | tui table)]` builds the same
/// value on a slow disk as on a fast one:
///
/// - an external command's output stays live (`tail -f log | tui log`);
/// - anything else is collected in full, or up to [`COLLECT_MAX_ITEMS`]
///   rows after which the rest stays live (`1.. | each { $in } | tui table`).
pub fn collect_input(data: PipelineData, span: Span) -> Option<Collected> {
    match data {
        PipelineData::ListStream(stream, _) => Some(collect_values(stream.into_iter(), span)),
        PipelineData::ByteStream(mut stream, _) => {
            let child_pid = match stream.source_mut() {
                ByteStreamSource::Child(child) => {
                    // Windows has no signal that means "the reader went
                    // away": a producer the TUI stops is force-killed, and
                    // that must not count as the command failing.
                    #[cfg(not(unix))]
                    child.ignore_error(true);
                    child.pid()
                }
                _ => None,
            };
            if stream.source().is_external() {
                let rx = spawn_reader(PipelineData::ByteStream(stream, None))?;
                return Some(Collected::Live {
                    stream: ListStream::new(receiver_values(rx), span, Signals::empty()),
                    child_pid,
                });
            }
            let lines = stream
                .lines()?
                .map_while(|line| line.ok())
                .map(move |text| Value::string(text, span));
            Some(collect_values(lines, span))
        }
        _ => None,
    }
}

/// Collect up to [`COLLECT_MAX_ITEMS`] values; the rest, if any, stays live.
fn collect_values(values: impl Iterator<Item = Value> + Send + 'static, span: Span) -> Collected {
    let mut values = values.peekable();
    let items: Vec<Value> = values.by_ref().take(COLLECT_MAX_ITEMS).collect();
    if values.peek().is_none() {
        return Collected::Done(items);
    }
    Collected::Live {
        stream: ListStream::new(items.into_iter().chain(values), span, Signals::empty()),
        child_pid: None,
    }
}

fn receiver_values(rx: Receiver<StreamMsg>) -> impl Iterator<Item = Value> + Send + 'static {
    rx.into_iter().map_while(|msg| match msg {
        StreamMsg::Value(value) => Some(value),
        StreamMsg::Done => None,
    })
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

/// Stop an external producer the TUI no longer reads. Without this the
/// pipeline would block on the command's exit status after the TUI closed
/// (`tail -f` never exits on its own).
pub fn kill_child(pid: u32) {
    // SIGPIPE says what happened (the reader went away) and the shell does
    // not report it as a failure, unlike SIGKILL.
    #[cfg(unix)]
    {
        const SIGPIPE: u32 = 13;
        let _ = nu_system::build_kill_command(false, std::iter::once(pid as i64), Some(SIGPIPE))
            .output();
    }
    #[cfg(not(unix))]
    {
        let _ = nu_system::kill_by_pid(pid as i64);
    }
}
