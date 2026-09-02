//! State shared between the DAP server thread and the nushell eval thread.
//!
//! CONCURRENCY RULE (deadlock hazard): the eval thread runs our `Debugger`
//! impl *while holding* the `EngineState.debugger` mutex, so the server thread
//! must NEVER touch `EngineState.debugger`. All communication goes through
//! this struct, which has its own `Arc` and its own locks.

use crate::dap::types::{StackFrame, Variable};
use crate::file_table::{FileId, FileTable};
use parking_lot::{Condvar, Mutex};
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::sync::Arc;

/// What the eval thread should do when it reaches the next instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RunMode {
    /// Run until a breakpoint (or end).
    Continue,
    /// Pause at the next instruction on a *different source line*, at
    /// block depth <= the recorded depth (i.e. don't stop inside callees).
    StepOver { depth: usize, line: u64 },
    /// Pause at the next instruction on a different line — or at any block
    /// depth change, so stepping INTO a closure/command whose body sits on
    /// the *same* source line still stops (`… | each {|n| $n * 2}`).
    StepIn { depth: usize, line: u64 },
    /// Pause at the next instruction at block depth < the recorded depth.
    StepOut { depth: usize },
    /// Pause at the very next instruction (used for stopOnEntry and pause).
    PauseNow,
}

/// One variable at pause time; children are flattened into
/// `PauseSnapshot::var_arena`.
#[derive(Debug, Clone)]
pub(crate) struct VarNode {
    pub var: Variable,
    /// Indices into the arena for this node's children (empty for leaves).
    pub children: Vec<usize>,
    /// Kept so `nuDapVisualize` can serve complete data even for leaves
    /// (strings, binaries) with no variablesReference of their own.
    pub value: nu_protocol::Value,
}

/// Everything the client may ask about while paused. Built by the eval thread
/// inside the Debugger callback, read by the server thread, replaced wholesale
/// on every pause.
#[derive(Debug, Default)]
pub(crate) struct PauseSnapshot {
    pub frames: Vec<StackFrame>,
    /// variablesReference -> children indices in `var_arena`. Low references
    /// are reserved for the scope roots (see the consts below).
    pub var_refs: HashMap<i64, Vec<usize>>,
    pub var_arena: Vec<VarNode>,
    next_ref: i64,
    /// Config as of this pause, so rendering uses nushell's own value
    /// formatting. Cloned from `engine_state` by the eval thread; the server
    /// thread only reads the `Arc`.
    pub config: Arc<nu_protocol::Config>,
    /// Engine-derived rendering facts, for the same reason as `config`.
    pub cache: Arc<RenderCache>,
}

/// Rendering facts that need `EngineState` to resolve, computed once after
/// the parse and shared by both threads because the server thread can't reach
/// `EngineState` (see the concurrency rule above).
///
/// Sized by the program, not the data: registering commands adds *decls*, not
/// blocks, so a small script yields a handful of entries.
#[derive(Debug, Default)]
pub(crate) struct RenderCache {
    /// Closure source text by block id, so a closure row reads as the literal
    /// the user wrote. Entries are capped, so a long body can't bloat the map.
    pub closure_src: HashMap<usize, String>,
    /// Labels for the capture rows under an expanded closure, by var id. Only
    /// captured vars, not every variable in the program.
    pub var_names: HashMap<usize, String>,
}

impl PauseSnapshot {
    pub(crate) const LOCALS_REF: i64 = 1;
    pub(crate) const PIPELINE_REF: i64 = 2;
    // 3 was the removed flat "Environment" scope; $env now lives in Globals.
    pub(crate) const REGISTERS_REF: i64 = 4;
    pub(crate) const PROCESS_REF: i64 = 5;
    pub(crate) const GLOBALS_REF: i64 = 6;

    pub(crate) fn new() -> Self {
        Self {
            frames: Vec::new(),
            var_refs: HashMap::new(),
            var_arena: Vec::new(),
            next_ref: 7, // 1..=6 reserved for scope roots
            config: Arc::default(),
            cache: Arc::default(),
        }
    }

    pub(crate) fn alloc_ref(&mut self) -> i64 {
        let r = self.next_ref;
        self.next_ref += 1;
        r
    }
}

/// Keyed in `SessionState::breakpoints` by its (possibly snapped) line.
#[derive(Debug, Clone, Default)]
pub(crate) struct Breakpoint {
    pub id: i64,
    pub verified: bool,
    /// nu expression; the breakpoint only pauses when it evaluates truthy.
    pub condition: Option<String>,
    /// Logpoint: emit this message (with {expr} interpolation) and keep going.
    pub log_message: Option<String>,
}

/// What a breakpoint does when execution arrives at it.
///
/// `condition` and `log_message` are independent fields (DAP sends them that
/// way), so this collapses them into the one behaviour that applies.
pub(crate) enum BpKind<'a> {
    /// Logpoint: emit the message, never pause. A condition, when the client
    /// sent one alongside the message, gates the logging.
    Log {
        template: &'a str,
        condition: Option<&'a str>,
    },
    /// Pause only when the expression evaluates truthy.
    Conditional(&'a str),
    /// Pause on arrival.
    Plain,
}

impl Breakpoint {
    pub(crate) fn kind(&self) -> BpKind<'_> {
        match (&self.log_message, &self.condition) {
            (Some(template), condition) => BpKind::Log {
                template,
                condition: condition.as_deref(),
            },
            (None, Some(cond)) => BpKind::Conditional(cond),
            (None, None) => BpKind::Plain,
        }
    }
}

/// The debug session's mutable state: breakpoints, run control, pause status,
/// the current variable snapshot, and the time-travel tape.
///
/// Holding a `&`/`&mut` to it means the lock is held: do not evaluate nu
/// expressions (`scratch` has its own lock) or block on a condvar while one is
/// alive.
#[derive(Debug)]
pub(crate) struct SessionState {
    /// file -> line -> properties, keyed by [`FileId`] so the client's
    /// spelling and nu's cannot disagree (see [`FileTable`]).
    pub breakpoints: HashMap<FileId, BTreeMap<i64, Breakpoint>>,
    /// file -> lines with at least one steppable instruction, populated by the
    /// eval thread after parsing.
    pub valid_lines: HashMap<FileId, BTreeSet<i64>>,
    /// True once `valid_lines` is populated (breakpoints can be verified).
    pub parse_done: bool,
    /// Monotonic id source for breakpoints.
    pub next_bp_id: i64,
    /// Whether to pause when a command raises an error.
    pub break_on_error: bool,
    /// (exceptionId, description) of the error we are currently paused on.
    pub exception_info: Option<(String, String)>,
    pub run_mode: RunMode,
    /// True while the eval thread is blocked inside a Debugger callback.
    pub paused: bool,
    /// Set by the server thread to release the eval thread.
    pub resume_requested: bool,
    /// Set on `disconnect`/`terminate`: the eval thread should bail out.
    pub terminate_requested: bool,
    /// Set on `restart`: this state's eval thread is being replaced, so its
    /// terminated/exited events are suppressed and the session survives.
    restarting: bool,
    pub snapshot: PauseSnapshot,
    /// Source line and block depth at the most recent pause; the server
    /// thread needs these to construct StepOver/StepOut run modes.
    pub paused_line: u64,
    pub paused_depth: usize,
    /// Current frame's locals by `VarId`, snapshotted from the real `Stack`
    /// (#18708), which the server thread must never touch itself.
    pub shadow_vars: HashMap<usize, ShadowVar>,
    /// Full runtime env (`stack.get_env_vars`), snapshotted alongside
    /// `shadow_vars` and rendered as `$env` in Globals.
    pub env_shadow: HashMap<String, nu_protocol::Value>,

    // --- Time-travel ("recorded tape") ---
    /// Recorded history, a bounded ring buffer whose last entry is the
    /// frontier (the live position).
    pub timeline: VecDeque<TimelineEntry>,
    /// Navigation cursor. `None` = at the live frontier (serve `snapshot`);
    /// `Some(i)` = viewing `timeline[i]` (serve `history_snapshot`).
    pub view_index: Option<usize>,
    /// Snapshot rebuilt from the timeline when viewing the past.
    pub history_snapshot: PauseSnapshot,
    /// Record every steppable line (true) vs. only pause points (false).
    pub time_travel: bool,
    /// Ring-buffer cap.
    tt_max: usize,
    /// Cached once by the eval thread so the server can rebuild historical
    /// Globals without `engine_state`.
    pub nu_constant: Option<nu_protocol::Value>,
    pub baseline_env: Option<HashMap<String, nu_protocol::Value>>,
    /// Cached alongside them, for the same reason: rebuilding a historical
    /// snapshot needs a `Config` to render values with.
    pub config: Arc<nu_protocol::Config>,
}

impl SessionState {
    /// Where a breakpoint at `line` lands, as (line, verified): the line
    /// itself if steppable, else the next one that is, else unverified. Before
    /// parsing, optimistically verified in place.
    pub(crate) fn snap_line(&self, file: FileId, line: i64) -> (i64, bool) {
        if !self.parse_done {
            return (line, true);
        }
        match self.valid_lines.get(&file) {
            Some(lines) => match lines.range(line..).next() {
                Some(&l) => (l, true),
                None => (line, false),
            },
            None => (line, false),
        }
    }

    /// Snapshot the client should be served right now: the rebuilt history
    /// view when scrubbing the past, else the live one.
    pub(crate) fn active_snapshot(&self) -> &PauseSnapshot {
        if self.view_index.is_some() {
            &self.history_snapshot
        } else {
            &self.snapshot
        }
    }

    pub(crate) fn active_snapshot_mut(&mut self) -> &mut PauseSnapshot {
        if self.view_index.is_some() {
            &mut self.history_snapshot
        } else {
            &mut self.snapshot
        }
    }

    /// Append a recorded moment, evicting the oldest when over the cap. The
    /// cursor shifts with the buffer so it keeps pointing at the same entry.
    pub(crate) fn push_timeline(&mut self, entry: TimelineEntry) {
        self.timeline.push_back(entry);
        while self.timeline.len() > self.tt_max {
            self.timeline.pop_front();
            if let Some(i) = self.view_index {
                self.view_index = Some(i.saturating_sub(1));
            }
        }
    }

    /// Index of the live frontier (last recorded entry), if any.
    pub(crate) fn frontier(&self) -> Option<usize> {
        self.timeline.len().checked_sub(1)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ShadowVar {
    pub name: String,
    pub value: nu_protocol::Value,
}

/// One recorded moment on the time-travel tape: enough to rebuild the Locals
/// and Globals view of a past line without `engine_state`. Recorded by the
/// eval thread at every steppable line, navigated by the server thread.
#[derive(Debug, Clone)]
pub(crate) struct TimelineEntry {
    /// Pre-resolved to file/line: the `SourceMap` lives on the eval thread.
    pub frames: Vec<StackFrame>,
    pub shadow_vars: HashMap<usize, ShadowVar>,
    pub env_shadow: HashMap<String, nu_protocol::Value>,
    pub last_result: Option<nu_protocol::Value>,
    /// At a pipe-stage boundary: (command name, value flowing in), shown as
    /// `in → cmd` in the past view's Pipeline scope.
    pub pipe_input: Option<(String, nu_protocol::Value)>,
    pub depth: usize,
    /// Whether execution actually paused here, for reverse-continue.
    pub is_breakpoint: bool,
}

/// Answer to a UI request (QuickPick / InputBox) shown by the client.
#[derive(Debug, Clone, Default)]
pub(crate) struct UiReply {
    pub cancelled: bool,
    pub index: Option<usize>,
    pub indices: Option<Vec<usize>>,
    pub value: Option<String>,
}

/// Bridge for interactive prompts: the eval thread, inside an `input`-family
/// shim, blocks here until the client answers a `nuDapUi` event with a
/// `nuDapUiReply` request.
#[derive(Debug, Default)]
pub(crate) struct UiBridge {
    pub replies: Mutex<HashMap<u64, UiReply>>,
    pub cv: Condvar,
    pub next_id: std::sync::atomic::AtomicU64,
}

pub(crate) struct DebugState {
    pub session_state: Mutex<SessionState>,
    /// The session's path <-> [`FileId`] table, shared with the `Session`
    /// rather than owned so a `restart`, which builds a fresh `DebugState` and
    /// copies the breakpoints across, keeps their ids valid.
    pub files: FileTable,
    pub ui: UiBridge,
    /// Lock-free mirror of `SessionState::terminate_requested`, polled by the
    /// UI wait loop so the stop button interrupts a pending dialog.
    pub terminate_flag: std::sync::atomic::AtomicBool,
    /// Scratch engine for watch/hover/console expressions, breakpoint
    /// conditions and logpoint interpolation. Cloned off the run engine right
    /// after the parse, so `None` only before the first run starts. Lock
    /// discipline: never taken while holding `session_state`.
    pub scratch: Mutex<Option<crate::eval_scratch::Scratch>>,
    /// Eval thread waits on this while paused; server thread notifies
    /// after setting `resume_requested` + new `run_mode`.
    pub resume_cv: Condvar,
    /// Signals "paused, snapshot ready". Unused so far: the `stopped` event
    /// is only sent once the snapshot is built.
    pub paused_cv: Condvar,
    /// Rendering facts, built once by the eval thread right after the parse
    /// and read by both.
    ///
    /// Lock discipline: innermost. Taken under `session_state` but never the
    /// other way round, and every use is a clone-and-release of the `Arc`.
    pub cache: Mutex<Arc<RenderCache>>,
}

impl DebugState {
    pub(crate) fn new(
        stop_on_entry: bool,
        time_travel: bool,
        tt_max: usize,
        files: FileTable,
    ) -> Self {
        Self {
            files,
            session_state: Mutex::new(SessionState {
                breakpoints: HashMap::new(),
                valid_lines: HashMap::new(),
                parse_done: false,
                next_bp_id: 1,
                break_on_error: true, // matches the filter's default:true
                exception_info: None,
                run_mode: if stop_on_entry {
                    RunMode::PauseNow
                } else {
                    RunMode::Continue
                },
                paused: false,
                resume_requested: false,
                terminate_requested: false,
                restarting: false,
                snapshot: PauseSnapshot::new(),
                paused_line: 0,
                paused_depth: 0,
                shadow_vars: HashMap::new(),
                env_shadow: HashMap::new(),
                timeline: VecDeque::new(),
                view_index: None,
                history_snapshot: PauseSnapshot::new(),
                time_travel,
                tt_max: tt_max.max(1),
                nu_constant: None,
                baseline_env: None,
                config: Arc::default(),
            }),
            ui: UiBridge::default(),
            terminate_flag: std::sync::atomic::AtomicBool::new(false),
            scratch: Mutex::new(None),
            resume_cv: Condvar::new(),
            paused_cv: Condvar::new(),
            cache: Mutex::new(Arc::default()),
        }
    }

    /// Called by the server thread to resume with a new run mode.
    pub(crate) fn resume(&self, mode: RunMode) {
        let mut session = self.session_state.lock();
        session.run_mode = mode;
        session.resume_requested = true;
        drop(session);
        self.resume_cv.notify_all();
    }

    pub(crate) fn request_terminate(&self) {
        let mut session = self.session_state.lock();
        session.terminate_requested = true;
        session.resume_requested = true;
        drop(session);
        self.terminate_flag
            .store(true, std::sync::atomic::Ordering::SeqCst);
        self.resume_cv.notify_all();
        self.ui.cv.notify_all();
    }

    /// Like `request_terminate`, but marks this state as being replaced by a
    /// restart so the dying eval thread keeps quiet about it.
    pub(crate) fn request_restart_teardown(&self) {
        let mut session = self.session_state.lock();
        session.restarting = true;
        session.terminate_requested = true;
        session.resume_requested = true;
        drop(session);
        self.terminate_flag
            .store(true, std::sync::atomic::Ordering::SeqCst);
        self.resume_cv.notify_all();
        self.ui.cv.notify_all();
    }

    pub(crate) fn is_restarting(&self) -> bool {
        self.session_state.lock().restarting
    }
}
