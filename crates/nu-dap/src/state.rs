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

/// Where a breakpoint sits: a line, and for an inline breakpoint the column
/// of the one instruction on it that should fire.
///
/// `column: None` is an ordinary gutter breakpoint (F9). It covers the whole
/// line: the first instruction reached on that line fires it, whatever column
/// that is — including a closure body sharing the line, which arrives in its
/// own frame and so counts as a fresh arrival.
///
/// `column: Some(_)` is an inline breakpoint (Shift+F9), bound to a single
/// instruction position. That is what lets one line carrying several steppable
/// positions — `$nums | each {|n| $n * 2 }`, where the pipeline stage and the
/// closure body are both on it — be broken on separately.
///
/// Ordering is line-then-column with `None` first, so a line's gutter
/// breakpoint sorts ahead of the inline ones on it and a `BTreeMap` keyed by
/// this can be ranged over per line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct BpPos {
    pub line: i64,
    pub column: Option<i64>,
}

impl BpPos {
    /// A whole-line (gutter) breakpoint position.
    pub(crate) fn line(line: i64) -> Self {
        Self { line, column: None }
    }
}

impl std::fmt::Display for BpPos {
    /// For the "already covered" message a client shows beside a breakpoint
    /// it had to grey out.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.column {
            Some(column) => write!(f, "line {}, column {column}", self.line),
            None => write!(f, "line {}", self.line),
        }
    }
}

/// Keyed in `SessionState::breakpoints` by its (possibly snapped) [`BpPos`].
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
    pub breakpoints: HashMap<FileId, BTreeMap<BpPos, Breakpoint>>,
    /// file -> `(line, column)` of every steppable instruction, populated by
    /// the eval thread after parsing. Sorted, so one set answers both
    /// questions asked of it: whether a line carries any instruction, and
    /// which column on it an inline breakpoint should bind to.
    pub valid_positions: HashMap<FileId, BTreeSet<(i64, i64)>>,
    /// True once `valid_positions` is populated (breakpoints can be verified).
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
    ///
    /// Behind an `Arc` so the time-travel tape can point at it instead of
    /// copying it: `$env` is large and changes rarely, so with time travel on
    /// (the default) every recorded line would otherwise carry its own clone
    /// of the whole environment.
    pub env_shadow: Arc<HashMap<String, nu_protocol::Value>>,

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
    /// Where a breakpoint request lands, as (position, verified).
    ///
    /// A request with no column snaps forward to the next line carrying
    /// instructions, as it always has. A request with one binds to a real
    /// instruction on the requested line: the first at or after the requested
    /// column, else the last on the line (the cursor sat past every statement
    /// on it), else — the line has nothing at all — it degrades to a
    /// whole-line breakpoint and snaps forward like one.
    ///
    /// Before the parse there are no positions to bind to, so the request is
    /// kept as asked and optimistically verified; the eval thread reconciles
    /// every breakpoint once the parse has produced the real positions (see
    /// `engine::publish_valid_positions`).
    pub(crate) fn snap(&self, file: FileId, line: i64, column: Option<i64>) -> (BpPos, bool) {
        if !self.parse_done {
            return (BpPos { line, column }, true);
        }

        let Some(positions) = self.valid_positions.get(&file) else {
            return (BpPos { line, column }, false);
        };

        let Some(column) = column else {
            return match positions.range((line, i64::MIN)..).next() {
                Some(&(line, _)) => (BpPos::line(line), true),
                None => (BpPos::line(line), false),
            };
        };

        // Positions on the requested line, in column order.
        let on_line = || positions.range((line, i64::MIN)..(line + 1, i64::MIN));

        if let Some(&(line, column)) = on_line().find(|(_, c)| *c >= column) {
            return (
                BpPos {
                    line,
                    column: Some(column),
                },
                true,
            );
        }

        if let Some(&(line, column)) = on_line().next_back() {
            return (
                BpPos {
                    line,
                    column: Some(column),
                },
                true,
            );
        }

        self.snap(file, line, None)
    }

    /// Every position on `line..=end_line` that can carry a breakpoint, for
    /// the client's `breakpointLocations` request. Empty before the parse.
    pub(crate) fn positions_in(&self, file: FileId, line: i64, end_line: i64) -> Vec<(i64, i64)> {
        self.valid_positions
            .get(&file)
            .map(|positions| {
                positions
                    .range((line, i64::MIN)..(end_line.saturating_add(1), i64::MIN))
                    .copied()
                    .collect()
            })
            .unwrap_or_default()
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

    /// Shadow variables the client should be served right now: the ones
    /// recorded at the entry being viewed when scrubbing the past, else the
    /// live ones. Mirrors [`Self::active_snapshot`] so that hover, watch and
    /// the Variables pane cannot disagree about which moment they describe.
    pub(crate) fn active_shadow_vars(&self) -> &HashMap<usize, ShadowVar> {
        self.view_index
            .and_then(|i| self.timeline.get(i))
            .map(|entry| &entry.shadow_vars)
            .unwrap_or(&self.shadow_vars)
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
    /// Shared with the live snapshot and with every other entry recorded while
    /// the environment did not change — see [`SessionState::env_shadow`].
    pub env_shadow: Arc<HashMap<String, nu_protocol::Value>>,
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
                valid_positions: HashMap::new(),
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
                env_shadow: Arc::default(),
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

#[cfg(test)]
mod tests {
    //! Unit tests for [`crate::state`].

    use super::{BpPos, DebugState};
    use crate::file_table::{FileId, FileTable};
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    /// Columns of `let doubled = ($nums | each {|n| $n * 2 })`: the statement,
    /// `$nums`, `each`, the closure literal, and `$n` inside its body.
    const LINE4: [(i64, i64); 5] = [(4, 1), (4, 16), (4, 24), (4, 29), (4, 34)];

    /// A post-parse session whose one file carries `positions`.
    fn parsed(positions: &[(i64, i64)]) -> (DebugState, FileId) {
        let files = FileTable::default();
        let file = files.intern("script.nu");
        let state = DebugState::new(false, false, 10, files);
        {
            let mut session = state.session_state.lock();
            session.parse_done = true;
            session
                .valid_positions
                .insert(file, positions.iter().copied().collect());
        }
        (state, file)
    }

    /// A gutter breakpoint (no column) keeps its long-standing behaviour: the
    /// line if it carries instructions, else the next line that does, else
    /// unverified where it was asked for.
    #[rstest]
    #[case::on_a_live_line(4, Some(BpPos::line(4)))]
    #[case::snaps_forward_over_a_blank_line(5, Some(BpPos::line(6)))]
    #[case::past_the_end_stays_put_unverified(9, None)]
    fn a_gutter_breakpoint_snaps_by_line(#[case] line: i64, #[case] expected: Option<BpPos>) {
        let mut positions = LINE4.to_vec();
        positions.push((6, 1));
        let (state, file) = parsed(&positions);
        let session = state.session_state.lock();

        let (pos, verified) = session.snap(file, line, None);
        match expected {
            Some(want) => {
                assert_eq!((pos, verified), (want, true));
            }
            None => {
                assert_eq!((pos, verified), (BpPos::line(line), false));
            }
        }
    }

    /// An inline breakpoint binds to one real instruction on the line: the
    /// first at or after the requested column, else the last on the line when
    /// the cursor sat past every statement.
    #[rstest]
    #[case::exactly_on_a_position(1, 1)]
    #[case::between_positions_takes_the_next(30, 34)]
    #[case::on_the_closure_body(34, 34)]
    #[case::past_every_statement_takes_the_last(99, 34)]
    fn an_inline_breakpoint_binds_to_a_position(#[case] asked: i64, #[case] bound: i64) {
        let (state, file) = parsed(&LINE4);
        let session = state.session_state.lock();

        assert_eq!(
            session.snap(file, 4, Some(asked)),
            (
                BpPos {
                    line: 4,
                    column: Some(bound)
                },
                true
            )
        );
    }

    /// A column on a line with no instructions at all has nothing to bind to,
    /// so it degrades to a gutter breakpoint and snaps forward like one —
    /// better than reporting it unverified where the user clicked.
    #[test]
    fn an_inline_breakpoint_on_a_dead_line_degrades_to_the_whole_line() {
        let mut positions = LINE4.to_vec();
        positions.push((6, 1));
        let (state, file) = parsed(&positions);
        let session = state.session_state.lock();

        assert_eq!(
            session.snap(file, 5, Some(12)),
            (BpPos::line(6), true),
            "should land on line 6 as a line breakpoint, column dropped"
        );
    }

    /// Before the parse there are no positions to bind to, so a request is
    /// kept exactly as asked and optimistically verified; the eval thread
    /// reconciles it once the real positions exist.
    #[rstest]
    #[case::gutter(None)]
    #[case::inline(Some(34))]
    fn before_the_parse_a_request_is_kept_as_asked(#[case] column: Option<i64>) {
        let files = FileTable::default();
        let file = files.intern("script.nu");
        let state = DebugState::new(false, false, 10, files);
        let session = state.session_state.lock();

        assert_eq!(
            session.snap(file, 4, column),
            (BpPos { line: 4, column }, true)
        );
    }

    /// `breakpointLocations` is served from the same set, over a line range.
    #[test]
    fn positions_in_covers_the_requested_line_range() {
        let mut positions = LINE4.to_vec();
        positions.push((6, 1));
        let (state, file) = parsed(&positions);
        let session = state.session_state.lock();

        assert_eq!(session.positions_in(file, 4, 4), LINE4.to_vec());
        assert_eq!(session.positions_in(file, 5, 6), vec![(6, 1)]);
        assert_eq!(session.positions_in(file, 7, 9), Vec::new());
    }
}
