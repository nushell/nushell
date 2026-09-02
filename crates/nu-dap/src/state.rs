//! State shared between the DAP server thread and the nushell eval thread.
//!
//! CONCURRENCY RULE (important, deadlock hazard):
//! The eval thread runs our `Debugger` impl *while holding* the
//! `EngineState.debugger` mutex (that lock is taken by `WithDebug` before
//! every callback). Therefore the DAP server thread must NEVER touch
//! `EngineState.debugger`. All communication goes through this struct,
//! which lives in its own `Arc` and has its own locks.

use crate::dap::types::{StackFrame, Variable};
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::sync::{Arc, Condvar, Mutex};

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

/// A snapshot of one variable at pause time. Children are pre-flattened
/// into the arena (see `PauseSnapshot::var_arena`).
#[derive(Debug, Clone)]
pub(crate) struct VarNode {
    pub(crate) var: Variable,
    /// Indices into the arena for this node's children (empty for leaves).
    pub(crate) children: Vec<usize>,
    /// Full value, kept so `nuDapVisualize` can serve complete data even for
    /// leaves (strings, binaries) that have no variablesReference of their own.
    pub(crate) value: nu_protocol::Value,
}

/// Everything the DAP client may ask about while we are paused.
/// Built by the eval thread inside the Debugger callback, read by the
/// server thread. Replaced wholesale on every pause.
#[derive(Debug, Default)]
pub(crate) struct PauseSnapshot {
    pub(crate) frames: Vec<StackFrame>,
    /// variablesReference -> children indices in `var_arena`.
    /// Reference 1 is reserved for the "Locals" scope root,
    /// reference 2 for the "Pipeline" scope root.
    pub(crate) var_refs: HashMap<i64, Vec<usize>>,
    pub(crate) var_arena: Vec<VarNode>,
    next_ref: i64,
    /// Config as of this pause, so rendering (`variables::short_render`) can
    /// use nushell's own value formatting. Cloned from `engine_state` by the
    /// eval thread; the server thread only ever reads this `Arc`.
    pub(crate) config: Arc<nu_protocol::Config>,
    /// Engine-derived rendering facts — same deal as `config`: resolving a
    /// `BlockId` or `VarId` needs `engine_state`, which the server can't reach.
    pub(crate) cache: Arc<RenderCache>,
}

/// The engine-derived facts rendering needs, resolved once after the script is
/// parsed and shared by both threads. The server thread can't reach
/// `EngineState` (see the concurrency rule above), so anything that needs a
/// `BlockId` or `VarId` looked up has to arrive pre-computed.
///
/// Sized by the program, not the data: registering commands adds *decls*, not
/// blocks, so a small script yields a handful of entries.
#[derive(Debug, Default)]
pub(crate) struct RenderCache {
    /// Closure source text by block id, so a closure row can read as the
    /// literal the user wrote. Each entry is capped, so a long closure body
    /// can't bloat the map.
    pub(crate) closure_src: HashMap<usize, String>,
    /// Names of variables that some closure captures, by var id — the labels
    /// for the capture rows under an expanded closure. Only captured vars are
    /// stored, not every variable in the program.
    pub(crate) var_names: HashMap<usize, String>,
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

/// One breakpoint's properties, keyed in `SessionState::breakpoints` by its
/// (possibly snapped) line.
#[derive(Debug, Clone, Default)]
pub(crate) struct Breakpoint {
    pub(crate) id: i64,
    pub(crate) verified: bool,
    /// nu expression; the breakpoint only pauses when it evaluates truthy.
    pub(crate) condition: Option<String>,
    /// Logpoint: emit this message (with {expr} interpolation) and keep going.
    pub(crate) log_message: Option<String>,
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
/// This is the portion of `DebugState` that lives behind its mutex, so both
/// threads reach it the same way — the eval thread via `DebugState::inner`,
/// the server thread via `Server::with_state`/`with_state_mut`. Holding a
/// `&`/`&mut` to it means the lock is held: do not evaluate nu expressions
/// (`scratch` has its own lock) or block on a condvar while one is alive.
#[derive(Debug)]
pub(crate) struct SessionState {
    /// file path (canonicalized) -> line -> breakpoint properties.
    pub(crate) breakpoints: HashMap<String, BTreeMap<i64, Breakpoint>>,
    /// file path -> set of lines that have at least one steppable
    /// instruction. Populated by the eval thread after parsing.
    pub(crate) valid_lines: HashMap<String, BTreeSet<i64>>,
    /// True once `valid_lines` is populated (breakpoints can be verified).
    pub(crate) parse_done: bool,
    /// Monotonic id source for breakpoints.
    pub(crate) next_bp_id: i64,
    /// Whether to pause when a command raises an error.
    pub(crate) break_on_error: bool,
    /// (exceptionId, description) of the error we are currently paused on.
    pub(crate) exception_info: Option<(String, String)>,
    pub(crate) run_mode: RunMode,
    /// True while the eval thread is blocked inside a Debugger callback.
    pub(crate) paused: bool,
    /// Set by the server thread to release the eval thread.
    pub(crate) resume_requested: bool,
    /// Set on `disconnect`/`terminate`: the eval thread should bail out.
    pub(crate) terminate_requested: bool,
    /// Set on `restart`: this state's eval thread is being torn down to be
    /// replaced. Suppresses its terminated/exited events so the DAP session
    /// survives the swap.
    restarting: bool,
    pub(crate) snapshot: PauseSnapshot,
    /// Source line and block depth at the most recent pause; the server
    /// thread needs these to construct StepOver/StepOut run modes.
    pub(crate) paused_line: u64,
    pub(crate) paused_depth: usize,
    /// Current frame's locals (keyed by VarId), snapshotted from the real
    /// `Stack` (#18708) by `debugger::sync_locals_from_stack` and read here by
    /// the server thread, which must never touch the Stack directly.
    pub(crate) shadow_vars: HashMap<usize, ShadowVar>,
    /// The current frame's full runtime environment (`stack.get_env_vars`),
    /// snapshotted alongside `shadow_vars`. Rendered as `$env` in Globals.
    pub(crate) env_shadow: HashMap<String, nu_protocol::Value>,

    // --- Time-travel ("recorded tape") ---
    /// Recorded execution history. Bounded ring buffer: the frontier (live
    /// position) is the last entry.
    pub(crate) timeline: VecDeque<TimelineEntry>,
    /// Navigation cursor. `None` = at the live frontier (serve `snapshot`);
    /// `Some(i)` = viewing `timeline[i]` (serve `history_snapshot`).
    pub(crate) view_index: Option<usize>,
    /// Snapshot rebuilt from the timeline when viewing the past.
    pub(crate) history_snapshot: PauseSnapshot,
    /// Record every steppable line (true) vs. only pause points (false).
    pub(crate) time_travel: bool,
    /// Ring-buffer cap.
    tt_max: usize,
    /// `$nu` constant and baseline env, cached once by the eval thread so the
    /// server can rebuild historical Globals without `engine_state`.
    pub(crate) nu_constant: Option<nu_protocol::Value>,
    pub(crate) baseline_env: Option<HashMap<String, nu_protocol::Value>>,
    /// Cached alongside them, for the same reason: rebuilding a historical
    /// snapshot needs a `Config` to render values with.
    pub(crate) config: Arc<nu_protocol::Config>,
}

impl SessionState {
    /// Where a breakpoint at `line` actually lands. Returns (line, verified):
    /// the line itself if steppable, else the next line that is (snap forward),
    /// else unverified. Optimistically verified in place before parsing.
    pub(crate) fn snap_line(&self, path: &str, line: i64) -> (i64, bool) {
        if !self.parse_done {
            return (line, true);
        }
        match self.valid_lines.get(path) {
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

    /// Append a recorded moment, evicting the oldest when over the cap. On
    /// eviction the cursor shifts with the buffer so it keeps pointing at the
    /// same logical entry (saturating at 0).
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
    pub(crate) name: String,
    pub(crate) value: nu_protocol::Value,
}

/// One recorded moment on the time-travel tape: everything needed to rebuild
/// the Locals + Globals view of a past line WITHOUT touching `engine_state`
/// (which the server thread must never do). Recorded on the eval thread at
/// every steppable line; navigated by the server thread.
#[derive(Debug, Clone)]
pub(crate) struct TimelineEntry {
    /// Stack frames already resolved to file/line (the server can't resolve
    /// spans — the SourceMap lives on the eval thread).
    pub(crate) frames: Vec<StackFrame>,
    pub(crate) shadow_vars: HashMap<usize, ShadowVar>,
    pub(crate) env_shadow: HashMap<String, nu_protocol::Value>,
    pub(crate) last_result: Option<nu_protocol::Value>,
    /// At a call/pipe-stage boundary: (command name, value flowing in) — shown
    /// as `in → cmd` in the past view's Pipeline scope.
    pub(crate) pipe_input: Option<(String, nu_protocol::Value)>,
    pub(crate) depth: usize,
    /// True if execution actually paused here on a breakpoint (used by
    /// reverse-continue to find prior stops).
    pub(crate) is_breakpoint: bool,
}

/// Answer to a UI request (QuickPick / InputBox) shown by the client.
#[derive(Debug, Clone, Default)]
pub(crate) struct UiReply {
    pub(crate) cancelled: bool,
    pub(crate) index: Option<usize>,
    pub(crate) indices: Option<Vec<usize>>,
    pub(crate) value: Option<String>,
}

/// Bridge for interactive prompts: the eval thread (inside an `input`-family
/// shim) blocks here until the client answers a `nuDapUi` event via the
/// `nuDapUiReply` request handled on the server thread.
#[derive(Debug, Default)]
pub(crate) struct UiBridge {
    pub(crate) replies: Mutex<HashMap<u64, UiReply>>,
    pub(crate) cv: Condvar,
    pub(crate) next_id: std::sync::atomic::AtomicU64,
}

pub(crate) struct DebugState {
    pub(crate) session_state: Mutex<SessionState>,
    pub(crate) ui: UiBridge,
    /// Mirror of SessionState::terminate_requested readable without the lock — the
    /// UI wait loop polls it so the stop button interrupts a pending dialog.
    pub(crate) terminate_flag: std::sync::atomic::AtomicBool,
    /// Scratch engine for watch/hover/console expressions, breakpoint
    /// conditions, and logpoint interpolation. Lazily initialized (building
    /// a full command context isn't free). Lock discipline: never taken
    /// while holding `inner`.
    pub(crate) scratch: Mutex<Option<crate::eval_scratch::Scratch>>,
    /// Eval thread waits on this while paused; server thread notifies
    /// after setting `resume_requested` + new `run_mode`.
    pub(crate) resume_cv: Condvar,
    /// Server thread can wait on this for "the eval thread has paused and
    /// the snapshot is ready" if it ever needs to (not required for v1:
    /// the `stopped` event is only sent after the snapshot is built).
    pub(crate) paused_cv: Condvar,
    /// Rendering facts, built once by the eval thread right after the parse
    /// (`engine::run`) and read by both.
    ///
    /// Lock discipline: innermost. It is taken under `session_state`
    /// (`build_snapshot`, `timetravel`) but never the other way round, and
    /// every use is a clone-and-release of the `Arc` — nothing is called while
    /// holding it, so it cannot be half of a cycle.
    pub(crate) cache: Mutex<Arc<RenderCache>>,
}

impl DebugState {
    pub(crate) fn new(stop_on_entry: bool, time_travel: bool, tt_max: usize) -> Self {
        Self {
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
        let mut session = self.session_state.lock().expect("session poisoned");
        session.run_mode = mode;
        session.resume_requested = true;
        drop(session);
        self.resume_cv.notify_all();
    }

    pub(crate) fn request_terminate(&self) {
        let mut session = self.session_state.lock().expect("session poisoned");
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
        let mut session = self.session_state.lock().expect("session poisoned");
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
        self.session_state
            .lock()
            .expect("session poisoned")
            .restarting
    }
}
