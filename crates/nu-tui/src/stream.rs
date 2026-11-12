//! Pipeline streams carried inside a `tui` value and read while it runs.
//!
//! Builders never read a stream. They keep it unread in the `tui` value as a
//! [`Feed`] and return at once, so `1.. | each { sleep 1sec } | tui log`
//! builds instantly and `tui run` shows each row as it is produced. Whether a
//! stream is fast, slow, finite or endless makes no difference. A stream
//! piped into a builder inside a child list (`[(ls | tui table)]`) rides in
//! that child's widgets, so it survives the list collecting the `tui` value.
use crate::app::TuiApp;
use nu_protocol::engine::EngineState;
use nu_protocol::{ByteStreamSource, ListStream, PipelineData, Signals, Span, Value};
use std::fmt;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Rows the reader threads may run ahead of the UI before they block, and
/// the most one poll hands over, so a fast producer cannot hold up a frame.
const CHANNEL_CAPACITY: usize = 8192;

/// A pipeline stream inside a `tui` value. Clones share it. The first TUI
/// to run reads the stream; if it reads it to the end, the newest rows stay
/// here and later runs show them again, so a saved `tui` value can be run
/// more than once.
#[derive(Clone)]
pub struct Feed(Arc<Mutex<Option<Source>>>);

/// What a [`Feed`] holds. It is empty while a TUI reads the stream, and
/// stays empty if the TUI closed before the stream ended.
enum Source {
    /// Not read yet.
    Stream {
        stream: ListStream,
        producer: Producer,
    },
    /// The rows of a stream an earlier TUI read to its end.
    Replay(Vec<Value>),
}

/// What produces a stream, to stop when the TUI closes before it ends.
#[derive(Clone, Copy)]
enum Producer {
    /// An external command whose output was piped straight in.
    Child(u32),
    /// A list stream, which may be wrapping an external command
    /// (`^tail -f app.log | lines`) whose pid it does not expose.
    Hidden,
    /// Nothing to stop: a file, or rows read by an earlier TUI.
    Nothing,
}

impl fmt::Debug for Feed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Feed")
    }
}

impl Feed {
    /// Keep a list or byte stream unread; a byte stream is read as lines.
    /// `None` for anything else, or for a child process without a stdout
    /// pipe (nothing to read).
    pub fn from_pipeline(data: PipelineData, span: Span) -> Option<Self> {
        let (stream, producer) = match data {
            PipelineData::ListStream(stream, _) => (stream, Producer::Hidden),
            PipelineData::ByteStream(mut stream, _) => {
                let producer = match stream.source_mut() {
                    ByteStreamSource::Child(child) => {
                        // Windows has no signal that means "the reader went
                        // away": a producer the TUI stops is force-killed,
                        // and that must not count as the command failing.
                        #[cfg(not(unix))]
                        child.ignore_error(true);
                        child.pid().map_or(Producer::Hidden, Producer::Child)
                    }
                    _ => Producer::Nothing,
                };
                let lines = stream
                    .lines()?
                    .map_while(|line| line.ok())
                    .map(move |text| Value::string(text, span));
                (ListStream::new(lines, span, Signals::empty()), producer)
            }
            _ => return None,
        };
        Some(Self(Arc::new(Mutex::new(Some(Source::Stream {
            stream,
            producer,
        })))))
    }

    /// Whether two feeds share one stream: a child list's stream given to
    /// each of its root widgets, or a saved `tui` value used twice.
    pub fn same(&self, other: &Feed) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }

    /// Whether running the TUI would show rows from it: the stream is
    /// unread, or an earlier TUI read it to the end.
    pub fn has_rows(&self) -> bool {
        self.0.lock().is_ok_and(|slot| slot.is_some())
    }

    /// What a TUI reads: the stream itself, taken so that it is read once,
    /// or a copy of the rows an earlier TUI read.
    fn take(&self) -> Option<Source> {
        let mut slot = self.0.lock().ok()?;
        match slot.as_ref()? {
            Source::Replay(rows) => Some(Source::Replay(rows.clone())),
            Source::Stream { .. } => slot.take(),
        }
    }

    /// Keep the rows of a stream that was read to its end, for later runs.
    fn keep(&self, rows: Vec<Value>) {
        if let Ok(mut slot) = self.0.lock() {
            *slot = Some(Source::Replay(rows));
        }
    }
}

/// Drop the oldest rows of a stream's list once it holds more than `cap`,
/// an eighth of `cap` at a time, so a list at the cap is not shifted on
/// every new row. What stays depends only on how many rows arrived, not on
/// how they were batched, so `tui run` and `tui debug` keep the same rows.
/// Returns how many were dropped.
pub fn trim_front(rows: &mut Vec<Value>, cap: usize) -> usize {
    let batch = (cap / 8).max(1);
    let over = rows.len().saturating_sub(cap);
    let dropped = over - over % batch;
    rows.drain(..dropped);
    dropped
}

enum StreamMsg {
    Value(Value),
    Done,
}

/// A stream being read on its own thread.
struct Reader {
    /// It fills the shared data: the outer pipeline's stream.
    shared: bool,
    /// Widgets whose own data it fills: a child list's stream.
    owners: Vec<String>,
    producer: Producer,
    done: bool,
}

/// Every stream a running TUI reads: the outer pipeline's, and any piped
/// into a child list. Dropping it stops them.
pub struct Readers {
    readers: Vec<Reader>,
    /// Rows from every reader thread, tagged with the reader's index.
    /// `None` once stopped.
    rx: Option<Receiver<(usize, StreamMsg)>>,
    /// The process group of the pipeline's external commands.
    pipeline_group: Arc<(AtomicU32, AtomicU32)>,
}

impl Readers {
    /// Start reading each stream `app` carries on a thread of its own.
    pub fn open(app: &TuiApp, engine_state: &EngineState) -> Self {
        // One reader per distinct stream: `$t | tui split [$t]` shows the
        // same stream as the shared data and in the child's table.
        let mut feeds: Vec<(Feed, bool, Vec<String>)> = Vec::new();
        if let Some(feed) = &app.stream {
            feeds.push((feed.clone(), true, Vec::new()));
        }
        for w in app.iter() {
            let Some(feed) = &w.stream else {
                continue;
            };
            match feeds.iter_mut().find(|(f, ..)| f.same(feed)) {
                Some((.., owners)) => owners.push(w.id.clone()),
                None => feeds.push((feed.clone(), false, vec![w.id.clone()])),
            }
        }
        let cap = app.stream_row_cap();
        let (tx, rx) = mpsc::sync_channel(CHANNEL_CAPACITY);
        let mut readers = Vec::new();
        for (feed, shared, owners) in feeds {
            let Some(source) = feed.take() else {
                continue;
            };
            let producer = match &source {
                Source::Stream { producer, .. } => *producer,
                Source::Replay(_) => Producer::Nothing,
            };
            let started = spawn_reader(readers.len(), source, feed, cap, tx.clone());
            readers.push(Reader {
                shared,
                owners,
                producer,
                done: !started,
            });
        }
        Self {
            readers,
            rx: Some(rx),
            pipeline_group: engine_state.pipeline_externals_state.clone(),
        }
    }

    /// Whether any stream is still producing.
    pub fn is_live(&self) -> bool {
        self.readers.iter().any(|r| !r.done)
    }

    /// Hand the rows that have arrived to `append`, with the widgets they
    /// belong to (empty for the shared data). Waits up to `wait` for the
    /// first one.
    pub fn poll(&mut self, wait: Duration, mut append: impl FnMut(&[String], Vec<Value>)) {
        let Some(rx) = &self.rx else {
            return;
        };
        let first = match rx.recv_timeout(wait) {
            Ok(msg) => msg,
            Err(RecvTimeoutError::Timeout) => return,
            Err(RecvTimeoutError::Disconnected) => {
                // Every reader thread has ended, including any whose
                // producer panicked before it could send `Done`.
                for reader in &mut self.readers {
                    reader.done = true;
                }
                return;
            }
        };
        let mut batches: Vec<Vec<Value>> = vec![Vec::new(); self.readers.len()];
        let arrived = std::iter::once(first).chain(std::iter::from_fn(|| rx.try_recv().ok()));
        for (index, msg) in arrived.take(CHANNEL_CAPACITY) {
            match msg {
                StreamMsg::Value(value) => {
                    if let Some(batch) = batches.get_mut(index) {
                        batch.push(value);
                    }
                }
                StreamMsg::Done => {
                    if let Some(reader) = self.readers.get_mut(index) {
                        reader.done = true;
                    }
                }
            }
        }
        for (reader, values) in self.readers.iter().zip(batches) {
            if values.is_empty() {
                continue;
            }
            match (reader.shared, reader.owners.is_empty()) {
                (true, false) => {
                    append(&[], values.clone());
                    append(&reader.owners, values);
                }
                (true, true) => append(&[], values),
                (false, _) => append(&reader.owners, values),
            }
        }
    }

    /// Stop reading, and stop the external commands behind streams that
    /// are still open. Without this the pipeline would block on their exit
    /// status after the TUI closed (`tail -f` never exits on its own).
    ///
    /// A reader thread ends after the row it is producing: nushell code in
    /// the stream (an `each` closure) cannot be interrupted mid-row.
    fn stop(&mut self) {
        // With the receiver gone, each thread's next send fails and it ends.
        self.rx = None;
        let open = self.readers.iter().filter(|r| !r.done);
        let mut hidden = false;
        for reader in open {
            match reader.producer {
                Producer::Child(pid) => kill(i64::from(pid)),
                Producer::Hidden => hidden = true,
                Producer::Nothing => {}
            }
        }
        // A list stream does not expose the pid of an external command it
        // wraps (`^tail -f app.log | lines`). An interactive shell starts a
        // pipeline's externals in a process group of their own, so stop
        // that group. It is 0 elsewhere (scripts, `nu -c`, Windows): there
        // the command stops the next time it writes, as it would when any
        // command stops reading a pipe.
        let group = self.pipeline_group.0.load(Ordering::SeqCst);
        if hidden && group != 0 && cfg!(unix) {
            kill(-i64::from(group));
        }
    }
}

impl Drop for Readers {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Read `source` on a thread of its own, sending its rows tagged with
/// `index`. A stream read to its end leaves its newest `cap` rows in `feed`
/// for later runs. Returns whether the thread started.
fn spawn_reader(
    index: usize,
    source: Source,
    feed: Feed,
    cap: usize,
    tx: SyncSender<(usize, StreamMsg)>,
) -> bool {
    // A producer blocked inside `next()` (an external command that never
    // writes) cannot be interrupted; `Readers::stop` kills it instead.
    thread::Builder::new()
        .name("nu-tui-stream".into())
        .spawn(move || {
            let (values, mut kept): (Box<dyn Iterator<Item = Value> + Send>, _) = match source {
                Source::Stream { stream, .. } => (Box::new(stream.into_iter()), Some(Vec::new())),
                Source::Replay(rows) => (Box::new(rows.into_iter()), None),
            };
            for value in values {
                if let Some(kept) = &mut kept {
                    kept.push(value.clone());
                    trim_front(kept, cap);
                }
                if tx.send((index, StreamMsg::Value(value))).is_err() {
                    return;
                }
            }
            // Kept before `Done` is sent, so a TUI that saw the stream end
            // has left its rows for the next run.
            if let Some(kept) = kept {
                feed.keep(kept);
            }
            let _ = tx.send((index, StreamMsg::Done));
        })
        .is_ok()
}

/// Stop a process, or a process group when `pid` is negative.
fn kill(pid: i64) {
    // SIGPIPE says what happened (the reader went away) and the shell does
    // not report it as a failure, unlike SIGKILL.
    #[cfg(unix)]
    {
        const SIGPIPE: u32 = 13;
        let _ = nu_system::build_kill_command(false, std::iter::once(pid), Some(SIGPIPE)).output();
    }
    #[cfg(not(unix))]
    {
        let _ = nu_system::kill_by_pid(pid);
    }
}
