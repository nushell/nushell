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
use std::sync::{Condvar, Mutex};

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
    /// The full underlying value, kept so `nuDapVisualize` can serve
    /// complete data for any node — including leaves like strings and
    /// binaries, which have no variablesReference of their own.
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
    pub(crate) next_ref: i64,
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
        }
    }

    pub(crate) fn alloc_ref(&mut self) -> i64 {
        let r = self.next_ref;
        self.next_ref += 1;
        r
    }
}

/// One breakpoint's properties, keyed in `Inner::breakpoints` by its
/// (possibly snapped) line.
#[derive(Debug, Clone, Default)]
pub(crate) struct BpProps {
    pub(crate) id: i64,
    pub(crate) verified: bool,
    /// nu expression; the breakpoint only pauses when it evaluates truthy.
    pub(crate) condition: Option<String>,
    /// Logpoint: emit this message (with {expr} interpolation) and keep going.
    pub(crate) log_message: Option<String>,
}

#[derive(Debug)]
pub(crate) struct Inner {
    /// file path (canonicalized) -> line -> breakpoint properties.
    pub(crate) breakpoints: HashMap<String, BTreeMap<i64, BpProps>>,
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
    pub(crate) restarting: bool,
    pub(crate) snapshot: PauseSnapshot,
    /// Source line and block depth at the most recent pause; the server
    /// thread needs these to construct StepOver/StepOut run modes.
    pub(crate) paused_line: u64,
    pub(crate) paused_depth: usize,
    /// The current frame's locals, keyed by nu VarId's inner usize. Snapshotted
    /// from the real evaluation `Stack` at each pause/record point (nushell
    /// #18708) by `debugger::sync_locals_from_stack`; read here by the server
    /// thread, which must never touch the Stack directly.
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
    pub(crate) tt_max: usize,
    /// `$nu` constant and baseline env, cached once by the eval thread so the
    /// server can rebuild historical Globals without `engine_state`.
    pub(crate) nu_constant: Option<nu_protocol::Value>,
    pub(crate) baseline_env: Option<HashMap<String, nu_protocol::Value>>,
}

impl Inner {
    /// Where should a breakpoint requested at `line` in `path` actually live?
    /// Returns (line, verified): the line itself if it has instructions, the
    /// next line that does (like other debuggers snap forward), or the
    /// original line unverified when nothing follows. Before parsing
    /// completes we optimistically verify in place.
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
    pub(crate) inner: Mutex<Inner>,
    pub(crate) ui: UiBridge,
    /// Mirror of Inner::terminate_requested readable without the lock — the
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
}

impl DebugState {
    pub(crate) fn new(stop_on_entry: bool, time_travel: bool, tt_max: usize) -> Self {
        Self {
            inner: Mutex::new(Inner {
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
            }),
            ui: UiBridge::default(),
            terminate_flag: std::sync::atomic::AtomicBool::new(false),
            scratch: Mutex::new(None),
            resume_cv: Condvar::new(),
            paused_cv: Condvar::new(),
        }
    }

    /// Called by the server thread to resume with a new run mode.
    pub(crate) fn resume(&self, mode: RunMode) {
        let mut inner = self.inner.lock().expect("debug state poisoned");
        inner.run_mode = mode;
        inner.resume_requested = true;
        drop(inner);
        self.resume_cv.notify_all();
    }

    pub(crate) fn request_terminate(&self) {
        let mut inner = self.inner.lock().expect("debug state poisoned");
        inner.terminate_requested = true;
        inner.resume_requested = true;
        drop(inner);
        self.terminate_flag
            .store(true, std::sync::atomic::Ordering::SeqCst);
        self.resume_cv.notify_all();
        self.ui.cv.notify_all();
    }

    /// Like `request_terminate`, but marks this state as being replaced by a
    /// restart so the dying eval thread keeps quiet about it.
    pub(crate) fn request_restart_teardown(&self) {
        let mut inner = self.inner.lock().expect("debug state poisoned");
        inner.restarting = true;
        inner.terminate_requested = true;
        inner.resume_requested = true;
        drop(inner);
        self.terminate_flag
            .store(true, std::sync::atomic::Ordering::SeqCst);
        self.resume_cv.notify_all();
        self.ui.cv.notify_all();
    }

    pub(crate) fn is_restarting(&self) -> bool {
        self.inner.lock().expect("debug state poisoned").restarting
    }
}
