//! `DapDebugger` — the bridge between nushell's evaluator and DAP.
//!
//! Implements `nu_protocol::debugger::Debugger`. The IR evaluator calls
//! `enter_instruction` before every instruction (while holding the
//! `EngineState.debugger` mutex — see the concurrency rule in state.rs).
//! That callback is where we:
//!   1. snapshot this frame's locals/env from the real `Stack` (nushell
//!      #18708) into shared state (`sync_locals_from_stack`),
//!   2. check breakpoints / step conditions,
//!   3. on pause: build a snapshot, emit `stopped`, and block this thread
//!      on a condvar until the DAP server thread resumes us.
//!
//! Termination: a Debugger cannot abort evaluation, but
//! `engine_state.signals().trigger()` raises the interrupt signal and the
//! evaluator bails out with `ShellError::Interrupted` at its next check.

mod snapshot;
pub(crate) mod stepping;

use crate::dap::protocol::DapWriter;
use crate::dap::types::DapEvent;
use crate::source_map::{SourceMap, SourcePos};
use crate::state::{BpKind, Breakpoint, DebugState, RunMode, SessionState};
use miette::Diagnostic;
use nu_protocol::ast::Block;
use nu_protocol::debugger::Debugger;
use nu_protocol::engine::{EngineState, Stack, StateWorkingSet};
use nu_protocol::ir::{Instruction, IrBlock};
use nu_protocol::{PipelineData, PipelineExecutionData, ShellError, Span, Value, format_cli_error};
use parking_lot::MutexGuard;
use std::sync::Arc;

#[derive(Debug)]
struct Frame {
    name: String,
    span: Option<Span>,
    /// Span of the instruction this frame is executing. For caller frames
    /// that is the call site, which is what a stack trace should show.
    at: Option<Span>,
    /// Last line executed *within this frame*, so returning from a callee
    /// onto the same line isn't a fresh arrival that re-fires breakpoints.
    last_line: Option<u64>,
    /// `$in` for this frame: register 0 at block entry. Register-based
    /// rather than on the Stack, so it is the one local read from IR.
    in_value: Option<Value>,
}

/// Where execution is: IR block, instruction index, register file. The
/// evaluator passes these three to every hook and they are never used apart.
struct Site<'a> {
    ir_block: &'a IrBlock,
    instruction_index: usize,
    registers: &'a [PipelineExecutionData],
}

impl<'a> Site<'a> {
    /// The instruction being executed.
    fn instruction(&self) -> &'a Instruction {
        &self.ir_block.instructions[self.instruction_index]
    }

    /// Its source span, possibly multi-line or synthetic: callers needing a
    /// stop location go through `SourceMap::resolve_steppable`.
    fn span(&self) -> Span {
        self.ir_block.spans[self.instruction_index]
    }
}

/// `Debug` is required by the `Debugger` trait. The three collaborators below
/// are skipped: they are not `Debug`, and dumping the shared state would print
/// every breakpoint, the timeline and the snapshot arena. Everything else
/// prints, so a field added later shows up on its own.
#[derive(derive_more::Debug)]
pub(crate) struct DapDebugger {
    #[debug(skip)]
    state: Arc<DebugState>,
    #[debug(skip)]
    writer: DapWriter,
    #[debug(skip)]
    source_map: SourceMap,
    /// Call-ish stack maintained from enter/leave_block. Depth 0 is the
    /// top-level script block.
    #[debug("{}", frames.len())]
    frames: Vec<Frame>,
    /// Span of the instruction we are currently paused/last stopped on,
    /// so frames[top] can report an accurate line.
    current_span: Option<Span>,
    /// Name for the next block, set at a `Call` to a named decl and consumed
    /// by `enter_block`. Cleared each `enter_instruction` so a builtin call
    /// can't mislabel a later block.
    pending_frame_name: Option<String>,
    /// Mirrors `frames.last().last_line`. A line compiles to several IR
    /// instructions, so breakpoints fire only when the line *changes*.
    last_line: Option<u64>,
    /// True while an error unwinds through nested frames: every frame's call
    /// instruction re-reports it, but only the innermost one should pause.
    in_error_unwind: bool,
    /// True between `enter_block` and its first instruction, where register 0
    /// still holds the block's pipeline input, i.e. `$in`.
    just_entered_block: bool,
    /// Most recent command/return result, shown as the `return` entry at the
    /// top of Locals. Captured in `leave_instruction`; streams are described,
    /// not drained.
    // Type only: the value itself can be a whole table.
    #[debug("{:?}", last_result.as_ref().map(|v| v.get_type()))]
    last_result: Option<Value>,
    /// Block depth at the previous steppable instruction, so time-travel
    /// records on depth changes too and matches step-into granularity.
    last_depth: Option<usize>,
}

impl DapDebugger {
    pub(crate) fn new(state: Arc<DebugState>, writer: DapWriter) -> Self {
        Self {
            source_map: SourceMap::new(state.files.clone()),
            state,
            writer,
            frames: Vec::new(),
            current_span: None,
            pending_frame_name: None,
            last_line: None,
            in_error_unwind: false,
            just_entered_block: false,
            last_result: None,
            last_depth: None,
        }
    }

    fn depth(&self) -> usize {
        self.frames.len()
    }

    /// The two `Arc`s a one-off render needs. Returned instead of a
    /// `RenderCtx` so the borrows outlive the call.
    fn render_parts(
        &self,
        engine_state: &EngineState,
    ) -> (Arc<nu_protocol::Config>, Arc<crate::state::RenderCache>) {
        (
            engine_state.get_config().clone(),
            self.state.cache.lock().clone(),
        )
    }

    /// Current shadow variables as (name, value) pairs for scratch eval.
    fn shadow_vars_for_eval(&self) -> Vec<(String, Value)> {
        let session = self.state.session_state.lock();
        session
            .shadow_vars
            .values()
            .map(|sv| (sv.name.clone(), sv.value.clone()))
            .collect()
    }

    fn scratch_eval(&self, expr: &str) -> Result<Value, String> {
        let vars = self.shadow_vars_for_eval();
        let mut guard = self.state.scratch.lock();

        guard
            .as_mut()
            .ok_or("no scratch engine: the run has not started")?
            .eval(expr, &vars)
    }

    fn scratch_interpolate(&self, template: &str) -> String {
        let vars = self.shadow_vars_for_eval();
        let mut guard = self.state.scratch.lock();

        // Nothing to interpolate against before the run starts: log the
        // template as written rather than dropping the message.
        match guard.as_mut() {
            Some(scratch) => scratch.interpolate(template, &vars),
            None => template.to_string(),
        }
    }

    /// The pause loop: publish snapshot, emit `stopped`, block until resumed.
    ///
    /// Takes the guard by value because the condvar wait needs it. Callers must
    /// therefore lock immediately before calling, once everything else that
    /// takes this lock has run — the mutex is not reentrant.
    fn pause(
        &self,
        mut session: MutexGuard<'_, SessionState>,
        engine_state: &EngineState,
        reason: &'static str,
        site: &Site<'_>,
        description: Option<&str>,
    ) {
        // So the Process scope tails are current.
        crate::stdio::flush_output(std::time::Duration::from_millis(300));

        self.announce_ir(engine_state, site);
        self.publish_stop(&mut session, engine_state, reason, site);
        self.emit_stopped(reason, description);

        while !session.resume_requested {
            self.state.resume_cv.wait(&mut session);
        }

        session.paused = false;
        session.resume_requested = false;

        if session.terminate_requested {
            // Evaluation unwinds with `Interrupted` at the next check.
            engine_state.signals().trigger();
        }
    }

    /// IR listing for the extension's "Show IR" panel. A custom event, so
    /// clients that don't know it ignore it.
    fn announce_ir(&self, engine_state: &EngineState, site: &Site<'_>) {
        self.writer.event(DapEvent::NuDapIr {
            text: format!("{}", site.ir_block.display(engine_state)),
            instruction_index: site.instruction_index,
            instruction_count: site.ir_block.instructions.len(),
        });
    }

    /// Record the stop in shared state — the snapshot and position the server
    /// serves `stackTrace`/`scopes`/`variables` from — then wake `paused_cv`.
    fn publish_stop(
        &self,
        session: &mut SessionState,
        engine_state: &EngineState,
        reason: &'static str,
        site: &Site<'_>,
    ) {
        if reason != "exception" {
            session.exception_info = None;
        }

        self.build_snapshot(engine_state, session, site);

        session.paused_line = self
            .current_span
            .and_then(|s| self.source_map.resolve(s))
            .map(|p| p.line)
            .unwrap_or(0);

        session.paused_depth = self.depth();
        session.paused = true;
        session.resume_requested = false;

        self.state.paused_cv.notify_all();
    }

    /// The DAP `stopped` event. Emitted only after `publish_stop`, so whatever
    /// the client fires back finds the snapshot already in place.
    fn emit_stopped(&self, reason: &'static str, description: Option<&str>) {
        self.writer.event(DapEvent::Stopped {
            reason,
            thread_id: crate::server::THREAD_ID,
            all_threads_stopped: true,
            description: description.map(String::from),
            text: description.map(String::from),
        });
    }

    /// First instruction of a fresh block: register 0 is `$in` (per element
    /// for each/where closures). Stashed on the frame for
    /// `sync_locals_from_stack` to inject, since it is not on the Stack.
    fn capture_block_input(&mut self, registers: &[PipelineExecutionData]) {
        if !self.just_entered_block {
            return;
        }

        self.just_entered_block = false;

        if let Some(PipelineData::Value(v, _)) = registers.first().map(|r| &r.body)
            && !matches!(v, Value::Nothing { .. })
            && let Some(frame) = self.frames.last_mut()
        {
            frame.in_value = Some(v.clone());
        }
    }

    /// Everything one instruction needs from shared state, under a single
    /// lock; conditions and logpoints are evaluated after it is released.
    /// `None` means terminate was requested and this instruction must stop.
    fn read_pause_gate(
        &self,
        engine_state: &EngineState,
        pos: Option<&SourcePos>,
    ) -> Option<PauseGate> {
        let session = self.state.session_state.lock();
        if session.terminate_requested {
            engine_state.signals().trigger();
            return None;
        }

        let bp_props = pos.and_then(|p| {
            // Only on first arrival at the line, not per instruction.
            if self.last_line == Some(p.line) {
                return None;
            }

            session
                .breakpoints
                .get(&p.file)
                .and_then(|m| m.get(&(p.line as i64)))
                .cloned()
        });

        Some(PauseGate {
            breakpoint: bp_props,
            run_mode: session.run_mode,
            time_travel: session.time_travel,
        })
    }

    /// Whether to stop here and why: a breakpoint's verdict wins, else the run
    /// mode decides. The second element is a console note for a breakpoint
    /// condition that could not be used.
    fn pause_reason(
        &mut self,
        engine_state: &EngineState,
        gate: &PauseGate,
        site: &Site<'_>,
        position: &SourcePos,
    ) -> (Option<&'static str>, Option<String>) {
        let (reason, note) = match &gate.breakpoint {
            Some(props) => self.check_breakpoint(engine_state, props),
            None => (None, None),
        };

        if reason.is_some() {
            return (reason, note);
        }

        let is_call = matches!(site.instruction(), Instruction::Call { .. });
        (
            self.should_pause_mode(position, gate.run_mode, is_call),
            note,
        )
    }

    /// Stop at this instruction: surface any breakpoint note, then pause.
    ///
    /// The only place `enter_instruction` takes the session lock, so everything
    /// else that takes it must have run already — the mutex is not reentrant.
    fn pause_at(
        &mut self,
        engine_state: &EngineState,
        site: &Site<'_>,
        reason: &'static str,
        note: Option<&str>,
    ) {
        if let Some(n) = note {
            self.writer.output("console", format!("nu-dap: {n}\n"));
        }

        let session = self.state.session_state.lock();
        self.pause(session, engine_state, reason, site, None);
    }

    /// Whether this breakpoint pauses, plus a console note when its condition
    /// could not be used. Requires locals to be synced already.
    fn check_breakpoint(
        &mut self,
        engine_state: &EngineState,
        props: &Breakpoint,
    ) -> (Option<&'static str>, Option<String>) {
        match props.kind() {
            BpKind::Log {
                template,
                condition,
            } => {
                if self.should_log(engine_state, condition) {
                    let msg = self.scratch_interpolate(template);
                    self.writer.output("console", format!("{msg}\n"));
                }
                (None, None)
            }
            BpKind::Conditional(cond) => match self.scratch_eval(cond) {
                Ok(Value::Bool { val: true, .. }) => (Some("breakpoint"), None),
                Ok(Value::Bool { val: false, .. }) => (None, None),
                // Pause on a broken condition rather than skip the breakpoint.
                Ok(v) => {
                    let (config, cache) = self.render_parts(engine_state);
                    let ctx = crate::variables::RenderCtx {
                        config: &config,
                        cache: &cache,
                    };
                    (
                        Some("breakpoint"),
                        Some(format!(
                            "condition `{cond}` returned {} (expected bool) — pausing",
                            crate::variables::short_render(&v, ctx)
                        )),
                    )
                }
                Err(e) => (
                    Some("breakpoint"),
                    Some(format!("condition `{cond}` failed: {e} — pausing")),
                ),
            },
            BpKind::Plain => (Some("breakpoint"), None),
        }
    }

    /// A logpoint's condition gates whether it emits. An unusable condition
    /// logs anyway, and says why: a logpoint never pauses, so a swallowed
    /// message would leave no sign of the failure.
    fn should_log(&mut self, engine_state: &EngineState, condition: Option<&str>) -> bool {
        let Some(cond) = condition else { return true };
        match self.scratch_eval(cond) {
            Ok(Value::Bool { val, .. }) => val,
            Ok(v) => {
                let (config, cache) = self.render_parts(engine_state);
                let rendered = crate::variables::short_render(
                    &v,
                    crate::variables::RenderCtx {
                        config: &config,
                        cache: &cache,
                    },
                );
                self.writer.output(
                    "console",
                    format!(
                        "nu-dap: log condition `{cond}` returned {rendered} (expected bool) — logging\n"
                    ),
                );
                true
            }
            Err(e) => {
                self.writer.output(
                    "console",
                    format!("nu-dap: log condition `{cond}` failed: {e} — logging\n"),
                );
                true
            }
        }
    }

    /// Record on the tape at the granularity forward stepping stops at
    /// (line/depth change or a call boundary), so Step Back reaches every point
    /// F11 would, pipe stages and same-line closure bodies included.
    ///
    /// Must run after `current_span` is set: `build_frames` reads it.
    fn record_timeline(
        &mut self,
        engine_state: &EngineState,
        site: &Site<'_>,
        pos: &SourcePos,
        reason: Option<&'static str>,
    ) {
        let depth = self.depth();
        let line_changed = self.last_line != Some(pos.line);
        let depth_changed = self.last_depth != Some(depth);
        let instruction = site.instruction();
        let is_call = matches!(instruction, Instruction::Call { .. });
        let pipe_input = pipe_input_at(engine_state, instruction, site.registers);

        let frames = self.build_frames();
        let last_result = self.last_result.clone();
        let mut session = self.state.session_state.lock();
        let granular = line_changed || depth_changed || is_call;

        if (session.time_travel && granular) || reason.is_some() {
            let entry = crate::state::TimelineEntry {
                frames,
                shadow_vars: session.shadow_vars.clone(),
                env_shadow: session.env_shadow.clone(),
                last_result,
                pipe_input,
                depth,
                is_breakpoint: reason == Some("breakpoint"),
            };
            session.push_timeline(entry);
            session.view_index = None; // execution advanced: back at the frontier
        }
    }

    /// Remember this line so later instructions on it don't refire the
    /// breakpoint. Only reached for steppable instructions, so compiler glue
    /// never touches line tracking.
    fn track_line(&mut self, pos: &SourcePos) {
        self.last_line = Some(pos.line);
        self.last_depth = Some(self.depth());
        if let Some(frame) = self.frames.last_mut() {
            frame.last_line = Some(pos.line);
        }
    }

    fn capture_last_return_value(&mut self, engine_state: &EngineState, site: &Site<'_>) {
        let reg = match *site.instruction() {
            Instruction::Call { src_dst, .. } => Some(src_dst.get() as usize),
            Instruction::Return { src } => Some(src.get() as usize),
            _ => None,
        };

        if let Some(idx) = reg
            && let Some(r) = site.registers.get(idx)
        {
            self.last_result = Some(match &r.body {
                PipelineData::Value(v, _) => v.clone(),
                PipelineData::Empty => Value::nothing(Span::unknown()),
                other => Value::string(
                    crate::variables::describe_stream(other, engine_state),
                    Span::unknown(),
                ),
            });
        }
    }

    /// Full diagnostic text for the `exceptionInfo` dialog: the report nushell
    /// prints to stderr, labels, help and source snippet included. `{err}`
    /// alone gives the top-level Display line only, while most `ShellError`
    /// variants put the substance in a `#[label]`. ANSI is stripped because DAP
    /// clients render escapes literally.
    fn error_report(engine_state: &EngineState, stack: &Stack, err: &ShellError) -> String {
        let working_set = StateWorkingSet::new(engine_state);
        nu_utils::strip_ansi_string_likely(format_cli_error(
            Some(stack),
            &working_set,
            err,
            Some("nu::shell::error"), // same default code as report_shell_error
        ))
    }

    /// Appends an external's stderr tail to either error text, so the dialog
    /// and the `stopped` event can't drift apart.
    fn with_external_stderr(text: String, tail: Option<&str>) -> String {
        match tail {
            Some(tail) => format!("{text}\n\n{tail}"),
            None => text,
        }
    }

    /// Trimmed and capped so the exception dialog stays readable. `None` when
    /// the child said nothing.
    fn get_message_from_external_command() -> Option<String> {
        crate::stdio::flush_output(std::time::Duration::from_millis(500));

        let tail = crate::stdio::recent_output("stderr");
        let tail = tail.trim();
        if tail.is_empty() {
            return None;
        }

        // Keep the dialog readable: last ~1000 chars.
        let start = tail.len().saturating_sub(1000);
        let mut cut = start;
        while !tail.is_char_boundary(cut) {
            cut += 1;
        }

        Some(tail[cut..].to_string())
    }

    fn handle_error(
        &mut self,
        engine_state: &EngineState,
        stack: &Stack,
        site: &Site<'_>,
        error: Option<&ShellError>,
    ) {
        let err = error.expect("error present");

        // Pause only at the innermost (first) report of the unwind.
        if self.in_error_unwind {
            return;
        }

        // Scoped: `sync_locals_from_stack` below takes this same lock.
        let wanted = {
            let session = self.state.session_state.lock();
            session.break_on_error && !session.terminate_requested
        };

        if !wanted {
            return;
        }

        self.in_error_unwind = true;

        // Point the top frame at the failing instruction, multi-line span and
        // all: an approximate position beats none.
        let span = site.span();
        if self.source_map.resolve(span).is_some() {
            self.current_span = Some(span);
        }

        // Fetched once (it flushes the output capture) and appended to both
        // texts below: "External command had a non-zero exit code" says
        // nothing on its own, the real complaint went to stderr. Matched on the
        // variant, not the code, so a rename fails to compile.
        let tail = match err {
            ShellError::NonZeroExitCode { .. } => Self::get_message_from_external_command(),
            _ => None,
        };
        let description = Self::with_external_stderr(
            Self::error_report(engine_state, stack, err),
            tail.as_deref(),
        );

        // This path skips the `enter_instruction` sync.
        self.sync_locals_from_stack(engine_state, stack);

        // Locked last and passed straight into `pause`: nothing in between
        // may take this lock again.
        let exception_id = exception_id(err);
        let mut session = self.state.session_state.lock();
        session.exception_info = Some((exception_id, description));

        // The `stopped` event's text lands in narrow client UI, so it gets the
        // short message; `exceptionInfo` serves the whole report above.
        let summary = Self::with_external_stderr(format!("{err}"), tail.as_deref());
        self.pause(session, engine_state, "exception", site, Some(&summary));
    }
}

/// The shared-state reads one instruction needs, taken together so the lock
/// is acquired once (see the concurrency rule in state.rs).
struct PauseGate {
    breakpoint: Option<Breakpoint>,
    run_mode: RunMode,
    time_travel: bool,
}

impl PauseGate {
    /// Locals+env are snapshotted from the Stack only when we might pause,
    /// evaluate a condition/logpoint, or record — never on the plain-`continue`
    /// path, so hot loops don't clone per line.
    fn wants_locals(&self) -> bool {
        self.breakpoint.is_some() || !matches!(self.run_mode, RunMode::Continue) || self.time_travel
    }
}

/// DAP `exceptionId` for an error: nushell's own diagnostic code, e.g.
/// `nu::shell::non_zero_exit_code`. Nearly every `ShellError` variant declares
/// a `code(..)`; `transparent` ones forward to their inner error.
///
/// Not scraped from `{err:?}`, because `Debug` is not a stable interface.
fn exception_id(err: &ShellError) -> String {
    err.code()
        .map(|code| code.to_string())
        // No variant should be missing a code, but an id is required.
        .unwrap_or_else(|| "nu::shell".to_string())
}

/// Frame naming only: a `Call` to a named decl labels the block its
/// `enter_block` pushes. Anything else yields `None`, so a builtin call (which
/// pushes no block) can't mislabel a later one.
fn called_decl_name(engine_state: &EngineState, instruction: &Instruction) -> Option<String> {
    match instruction {
        Instruction::Call { decl_id, .. } => {
            Some(engine_state.get_decl(*decl_id).name().to_string())
        }
        _ => None,
    }
}

/// At a call boundary, the value flowing in, for the past view's `in → cmd`
/// row. Streams are described, never drained.
fn pipe_input_at(
    engine_state: &EngineState,
    instruction: &Instruction,
    registers: &[PipelineExecutionData],
) -> Option<(String, Value)> {
    let (decl_id, src_dst) = match instruction {
        Instruction::Call {
            decl_id, src_dst, ..
        } => (decl_id, src_dst),
        _ => return None,
    };

    let name = || engine_state.get_decl(*decl_id).name().to_string();
    match &registers.get(src_dst.get() as usize)?.body {
        PipelineData::Value(v, _) if !matches!(v, Value::Nothing { .. }) => {
            Some((name(), v.clone()))
        }
        other @ (PipelineData::ListStream(..) | PipelineData::ByteStream(..)) => Some((
            name(),
            Value::string(
                crate::variables::describe_stream(other, engine_state),
                Span::unknown(),
            ),
        )),
        _ => None,
    }
}

impl Debugger for DapDebugger {
    fn enter_block(&mut self, engine_state: &EngineState, block: &Block) {
        self.source_map.refresh(engine_state);

        // Name from the preceding `Call` (custom command), else `file.nu:line`.
        let name = self.pending_frame_name.take().unwrap_or_else(|| {
            block
                .span
                .and_then(|s| self.source_map.resolve(s))
                .map(|p| {
                    format!(
                        "{}:{}",
                        {
                            let path = self.source_map.path(p.file);
                            std::path::Path::new(&path)
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or(path)
                        },
                        p.line
                    )
                })
                .unwrap_or_else(|| "block".to_string())
        });

        // The caller pauses at its call instruction while this block runs.
        if let Some(caller) = self.frames.last_mut() {
            caller.at = self.current_span;
        }

        self.frames.push(Frame {
            name,
            span: block.span,
            at: None,
            last_line: None,
            in_value: None,
        });
        self.last_line = None; // fresh frame: no line executed yet

        // Locals/params are read from the real Stack at each pause.
        self.just_entered_block = true;
    }

    fn leave_block(&mut self, _engine_state: &EngineState, _block: &Block) {
        self.frames.pop();
        // So returning onto the call line doesn't refire.
        self.last_line = self.frames.last().and_then(|f| f.last_line);
    }

    fn enter_instruction(
        &mut self,
        engine_state: &EngineState,
        stack: &Stack,
        ir_block: &IrBlock,
        instruction_index: usize,
        registers: &[PipelineExecutionData],
    ) {
        let site = Site {
            ir_block,
            instruction_index,
            registers,
        };

        self.source_map.refresh(engine_state);
        // A new instruction means a prior error was caught; future ones pause.
        self.in_error_unwind = false;
        self.capture_block_input(registers);
        self.pending_frame_name = called_decl_name(engine_state, site.instruction());

        // Only single-line spans are valid stop locations: block-wide glue
        // would make the UI jump to line 1.
        let span = site.span();
        let position = self.source_map.resolve_steppable(span);

        let gate = match self.read_pause_gate(engine_state, position.as_ref()) {
            Some(gate) => gate,
            // Terminate requested. Read before the position check below,
            // because it applies to every instruction, steppable or not.
            None => return,
        };

        // Nothing below applies without a stop location: breakpoints match
        // against a position, stepping needs a line to show, and the tape
        // records only real lines.
        let Some(position) = position else { return };

        if gate.wants_locals() {
            self.sync_locals_from_stack(engine_state, stack);
        }

        // Order is load-bearing from here: locals synced before a condition or
        // logpoint runs, `current_span` set before the tape entry is built,
        // line tracking updated only once any pause has returned.
        let (reason, note) = self.pause_reason(engine_state, &gate, &site, &position);

        self.current_span = Some(span);

        self.record_timeline(engine_state, &site, &position, reason);

        if let Some(r) = reason {
            self.pause_at(engine_state, &site, r, note.as_deref());
        }

        self.track_line(&position);
    }

    fn leave_instruction(
        &mut self,
        engine_state: &EngineState,
        stack: &Stack,
        ir_block: &IrBlock,
        instruction_index: usize,
        registers: &[PipelineExecutionData],
        error: Option<&ShellError>,
    ) {
        let site = Site {
            ir_block,
            instruction_index,
            registers,
        };

        if error.is_some() {
            self.handle_error(engine_state, stack, &site, error);
            return;
        }

        self.capture_last_return_value(engine_state, &site);
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for [`crate::debugger`].

    use super::exception_id;
    use nu_protocol::shell_error::generic::GenericError;
    use nu_protocol::{ShellError, Span};
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    /// The `exceptionId` is nushell's diagnostic code, not anything scraped
    /// out of `Debug`. Covers an explicit `code(..)` and a `transparent`
    /// variant that must forward to its inner error.
    #[rstest]
    #[case::non_zero_exit_code(
        ShellError::NonZeroExitCode {
            exit_code: std::num::NonZeroI32::new(42).expect("nonzero"),
            span: Span::unknown(),
        },
        "nu::shell::non_zero_exit_code"
    )]
    #[case::division_by_zero(
        ShellError::DivisionByZero { span: Span::unknown() },
        "nu::shell::division_by_zero"
    )]
    #[case::generic_forwards_to_inner(
        ShellError::Generic(GenericError::new("boom", "it broke", Span::unknown())),
        "nu::shell::error"
    )]
    fn exception_id_is_the_diagnostic_code(#[case] err: ShellError, #[case] expected: &str) {
        assert_eq!(exception_id(&err), expected);
    }

    /// A custom code on a `GenericError` reaches the client as-is: the id is
    /// the error's own identity, not one we invent.
    #[test]
    fn exception_id_honours_a_custom_code() {
        let err = ShellError::Generic(
            GenericError::new("boom", "it broke", Span::unknown()).with_code("nu::dap::made_up"),
        );
        assert_eq!(exception_id(&err), "nu::dap::made_up");
    }

    /// A required DAP field, so it must never come back empty, even if some
    /// variant ships without a `code(..)`.
    #[test]
    fn exception_id_is_never_empty() {
        let err = ShellError::Generic(GenericError::new(
            "boom",
            "a message with spaces, braces { } and parens ( )",
            Span::unknown(),
        ));
        let id = exception_id(&err);
        assert!(!id.is_empty(), "empty id");
        // The message must not leak into the identifier.
        assert!(!id.contains(' '), "id carries payload: {id}");
    }
}
