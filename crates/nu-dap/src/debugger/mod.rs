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
//! Termination: there is no API to abort evaluation from a Debugger, but
//! `engine_state.signals().trigger()` raises the interrupt signal, which
//! makes the evaluator bail out with `ShellError::Interrupted` at the next
//! check — that is our terminate path.

mod snapshot;
pub(crate) mod stepping;

use crate::dap::protocol::DapWriter;
use crate::source_map::{SourceMap, SourcePos};
use crate::state::{BpKind, Breakpoint, DebugState, RunMode, SessionState};
use nu_protocol::ast::Block;
use nu_protocol::debugger::Debugger;
use nu_protocol::engine::{EngineState, Stack};
use nu_protocol::ir::{Instruction, IrBlock};
use nu_protocol::{PipelineData, PipelineExecutionData, ShellError, Span, Value};
use serde_json::json;
use std::sync::{Arc, MutexGuard};

#[derive(Debug)]
struct Frame {
    name: String,
    span: Option<Span>,
    /// Span of the instruction this frame is currently executing — for
    /// caller frames that's the call site, which is what the stack trace
    /// should show (not the callee block's first line).
    at: Option<Span>,
    /// Last source line executed *within this frame*. Per-frame (not
    /// global) so returning from a callee back onto the same line doesn't
    /// count as a fresh arrival and re-fire breakpoints/logpoints.
    last_line: Option<u64>,
    /// `$in` for this frame: register 0 at block entry. `$in` is
    /// register-based, not stored on the Stack, so it's the one local we
    /// still capture from IR rather than read from `stack.vars`.
    in_value: Option<Value>,
}

/// Where execution currently is: the IR block, which instruction in it, and the
/// register file. The evaluator hands these to every hook as three separate
/// arguments and they are never used apart, so internally they travel as one.
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

    /// Its source span — may be multi-line or synthetic, so callers that need a
    /// stop location go through `SourceMap::resolve_steppable`.
    fn span(&self) -> Span {
        self.ir_block.spans[self.instruction_index]
    }
}

/// `Debug` is required by the `Debugger` trait. The three collaborators below
/// are skipped because they are not `Debug` themselves (and dumping the shared
/// state would print every breakpoint, the whole timeline, and the snapshot
/// arena); the rest print, so a field added later shows up without anyone
/// having to remember this impl.
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
    /// Name for the next block, set at a `Call` to a named decl. Consumed by
    /// `enter_block`; cleared each `enter_instruction` so a builtin call can't
    /// mislabel a later block.
    pending_frame_name: Option<String>,
    /// Previously executed source line in the current frame (mirrors
    /// frames.last().last_line). A line compiles to several IR instructions,
    /// so breakpoints fire only when the line *changes*.
    last_line: Option<u64>,
    /// True while an error is unwinding through nested frames. Each frame's
    /// call instruction re-reports the same error via `leave_instruction`;
    /// pause only on the innermost one. Cleared when a new instruction runs.
    in_error_unwind: bool,
    /// True between enter_block and its first instruction: register 0 holds
    /// the block's pipeline input there, which is `$in` — capture it.
    just_entered_block: bool,
    /// Result of the most recently completed command/return — surfaced as the
    /// `return` entry at the top of Locals ("latest expression result").
    /// Captured in `leave_instruction`; streams are described, not drained.
    // Type only: the value itself can be a whole table.
    #[debug("{:?}", last_result.as_ref().map(|v| v.get_type()))]
    last_result: Option<Value>,
    /// Block depth at the previous steppable instruction — lets time-travel
    /// recording fire on depth changes (entering/leaving closures), matching
    /// forward step-into granularity.
    last_depth: Option<usize>,
}

impl DapDebugger {
    pub(crate) fn new(state: Arc<DebugState>, writer: DapWriter) -> Self {
        Self {
            state,
            writer,
            source_map: SourceMap::default(),
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

    /// The two `Arc`s a one-off render needs, for callers that then build a
    /// `RenderCtx` borrowing them. Returned rather than the context itself so
    /// the borrows outlive the call.
    fn render_parts(
        &self,
        engine_state: &EngineState,
    ) -> (Arc<nu_protocol::Config>, Arc<crate::state::RenderCache>) {
        (
            engine_state.get_config().clone(),
            self.state
                .cache
                .lock()
                .expect("render cache poisoned")
                .clone(),
        )
    }

    /// Current shadow variables as (name, value) pairs for scratch eval.
    fn shadow_vars_for_eval(&self) -> Vec<(String, Value)> {
        let session = self.state.session_state.lock().expect("session poisoned");
        session
            .shadow_vars
            .values()
            .map(|sv| (sv.name.clone(), sv.value.clone()))
            .collect()
    }

    fn scratch_eval(&self, expr: &str) -> Result<Value, String> {
        let vars = self.shadow_vars_for_eval();
        let mut guard = self.state.scratch.lock().expect("scratch poisoned");

        guard
            .get_or_insert_with(crate::eval_scratch::Scratch::new)
            .eval(expr, &vars)
    }

    fn scratch_interpolate(&self, template: &str) -> String {
        let vars = self.shadow_vars_for_eval();
        let mut guard = self.state.scratch.lock().expect("scratch poisoned");

        guard
            .get_or_insert_with(crate::eval_scratch::Scratch::new)
            .interpolate(template, &vars)
    }

    /// The pause loop: publish snapshot, emit `stopped`, block until resumed.
    ///
    /// Takes the locked state by value because the condvar wait below consumes
    /// the guard — a `&mut SessionState` could not be handed to `wait`. Callers
    /// must therefore lock immediately before calling: everything that takes
    /// this lock (`read_pause_gate`, `sync_locals_from_stack`, `record_timeline`,
    /// `shadow_vars_for_eval` via breakpoint conditions/logpoints) has to run
    /// first. std mutexes are not reentrant, so a second lock here deadlocks.
    fn pause(
        &self,
        mut session: MutexGuard<'_, SessionState>,
        engine_state: &EngineState,
        reason: &'static str,
        site: &Site<'_>,
        description: Option<&str>,
    ) {
        // Sync the output capture so the Process scope tails are current.
        crate::stdio::flush_output(std::time::Duration::from_millis(300));

        self.announce_ir(engine_state, site);
        self.publish_stop(&mut session, engine_state, reason, site);
        self.emit_stopped(reason, description);

        while !session.resume_requested {
            session = self
                .state
                .resume_cv
                .wait(session)
                .expect("session poisoned");
        }

        session.paused = false;
        session.resume_requested = false;

        if session.terminate_requested {
            // Raise nu's interrupt signal: evaluation unwinds with
            // ShellError::Interrupted at the next signal check.
            engine_state.signals().trigger();
        }
    }

    /// IR listing for the extension's "Show IR" panel. Custom DAP event, so
    /// clients that don't know it simply ignore it.
    fn announce_ir(&self, engine_state: &EngineState, site: &Site<'_>) {
        self.writer.event(
            "nuDapIr",
            json!({
                "text": format!("{}", site.ir_block.display(engine_state)),
                "instructionIndex": site.instruction_index,
                "instructionCount": site.ir_block.instructions.len(),
            }),
        );
    }

    /// Record the stop in shared state — the snapshot and position the server
    /// thread serves `stackTrace`/`scopes`/`variables` from — then wake anyone
    /// waiting on `paused_cv`.
    ///
    /// Takes the locked state by reference; `pause` keeps the guard because the
    /// wait that follows consumes it.
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
        let mut body = json!({
            "reason": reason,
            "threadId": crate::server::THREAD_ID,
            "allThreadsStopped": true,
        });

        if let Some(text) = description {
            body["description"] = json!(text);
            body["text"] = json!(text);
        }

        self.writer.event("stopped", body);
    }

    /// First instruction of a fresh block: register 0 is `$in` (per element
    /// for each/where closures). It's register-based, not on the Stack, so
    /// stash it on the frame; `sync_locals_from_stack` injects it later.
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

    /// Everything one instruction needs from the shared state, read under a
    /// single lock — condition and logpoint evaluation happen after it is
    /// released (`scratch` has its own). `None` means terminate was requested
    /// and this instruction must not proceed.
    fn read_pause_gate(
        &self,
        engine_state: &EngineState,
        pos: Option<&SourcePos>,
    ) -> Option<PauseGate> {
        let session = self.state.session_state.lock().expect("session poisoned");
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
                .get(&p.path)
                .and_then(|m| m.get(&(p.line as i64)))
                .cloned()
        });

        Some(PauseGate {
            breakpoint: bp_props,
            run_mode: session.run_mode,
            time_travel: session.time_travel,
        })
    }

    /// Whether to stop at this instruction and why: a breakpoint's verdict wins,
    /// otherwise the active run mode decides. The second element is a console
    /// note from a breakpoint condition that could not be used.
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
    /// The only place `enter_instruction` takes the session lock. Everything
    /// that also takes it — `read_pause_gate`, `sync_locals_from_stack`,
    /// `check_breakpoint` (scratch eval), `record_timeline` — must have run
    /// already; std mutexes are not reentrant.
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

        let session = self.state.session_state.lock().expect("session poisoned");
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
    /// logs anyway (and says why): a logpoint never pauses, so swallowing the
    /// output would leave nothing to show that anything went wrong.
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

    /// Record on the tape at the same granularity forward stepping stops
    /// (line/depth change or a call boundary), so Step Back reaches every
    /// point F11 would — pipe stages and same-line closure bodies included.
    ///
    /// Must run after `current_span` is set: `build_frames` reads it for the
    /// top frame's line.
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
        let mut session = self.state.session_state.lock().expect("session poisoned");
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
    /// breakpoint. Only reached for steppable instructions, so line tracking
    /// stays untouched by compiler glue.
    fn track_line(&mut self, pos: &SourcePos) {
        self.last_line = Some(pos.line);
        self.last_depth = Some(self.depth());
        if let Some(frame) = self.frames.last_mut() {
            frame.last_line = Some(pos.line);
        }
    }

    fn capture_last_return_value(&mut self, site: &Site<'_>) {
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
                other => Value::string(crate::variables::describe_stream(other), Span::unknown()),
            });
        }
    }

    /// Dialog text for an error. External commands get their stderr tail
    /// appended: "External command had a non-zero exit code" says nothing on its
    /// own — the command's actual complaint went to stderr. Matched on the
    /// variant, not the diagnostic code, so a rename fails to compile instead of
    /// quietly dropping the tail.
    fn error_description(err: &ShellError) -> String {
        let mut description = format!("{err}");

        if matches!(err, ShellError::NonZeroExitCode { .. })
            && let Some(tail) = Self::get_message_from_external_command()
        {
            description = format!("{description}\n\n{tail}");
        }

        description
    }

    /// Recent stderr tail from an external command, trimmed and capped so the
    /// exception dialog stays readable. `None` when the child said nothing.
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

        // An error unwinds through every enclosing call instruction; pause
        // only at the innermost (first) report.
        if self.in_error_unwind {
            return;
        }

        // Scoped: `sync_locals_from_stack` below takes this same lock, so this
        // read must not outlive the check.
        let wanted = {
            let session = self.state.session_state.lock().expect("session poisoned");
            session.break_on_error && !session.terminate_requested
        };

        if !wanted {
            return;
        }

        self.in_error_unwind = true;

        // Point the top frame at the failing instruction, even when its span
        // is multi-line (better an approximate position than none).
        let span = site.span();
        if self.source_map.resolve(span).is_some() {
            self.current_span = Some(span);
        }

        let description = Self::error_description(err);

        // This path skips the enter_instruction sync, so refresh from the Stack.
        self.sync_locals_from_stack(engine_state, stack);

        // Locked last, and passed straight into `pause` — nothing between here
        // and the stop may take this lock again.
        let exception_id = exception_id(err);
        let mut session = self.state.session_state.lock().expect("session poisoned");
        session.exception_info = Some((exception_id, description.clone()));

        self.pause(session, engine_state, "exception", site, Some(&description));
    }
}

/// The shared-state reads one instruction needs, taken together so the lock is
/// acquired once (see the concurrency rule in state.rs).
struct PauseGate {
    breakpoint: Option<Breakpoint>,
    run_mode: RunMode,
    time_travel: bool,
}

impl PauseGate {
    /// Locals+env are snapshotted from the Stack only when we might pause, eval
    /// a condition/logpoint, or record — not on the plain-`continue` fast path,
    /// so hot loops don't clone per line.
    fn wants_locals(&self) -> bool {
        self.breakpoint.is_some() || !matches!(self.run_mode, RunMode::Continue) || self.time_travel
    }
}

/// DAP `exceptionId` for an error: nushell's own diagnostic code, e.g.
/// `nu::shell::non_zero_exit_code`. `ShellError` derives `Diagnostic` and
/// nearly every variant declares a `code(..)`; the `transparent` ones forward
/// to their inner error (`GenericError` defaults to `nu::shell::error`).
///
/// Deliberately not scraped from `{err:?}`: `Debug` is not a stable interface,
/// and this is the same code nushell prints for the error elsewhere.
pub(crate) fn exception_id(err: &ShellError) -> String {
    use miette::Diagnostic;
    err.code()
        .map(|code| code.to_string())
        // No variant should be missing a code, but an id is required.
        .unwrap_or_else(|| "nu::shell".to_string())
}

/// Frame naming only: a `Call` to a named decl labels the block its
/// `enter_block` will push; anything else yields `None` so a builtin call (no
/// block follows) can't mislabel a later block.
fn called_decl_name(engine_state: &EngineState, instruction: &Instruction) -> Option<String> {
    match instruction {
        Instruction::Call { decl_id, .. } => {
            Some(engine_state.get_decl(*decl_id).name().to_string())
        }
        _ => None,
    }
}

/// At a call boundary, the value flowing in — so the past view can show
/// `in → cmd`. Streams are described, never drained.
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
            Value::string(crate::variables::describe_stream(other), Span::unknown()),
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
                        std::path::Path::new(&p.path)
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| p.path.clone()),
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

        // Locals/params come from the real Stack at each pause, not pre-binding.
        self.just_entered_block = true;
    }

    fn leave_block(&mut self, _engine_state: &EngineState, _block: &Block) {
        self.frames.pop();
        // Restore the caller's line so returning onto the call line doesn't refire.
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
        // New instruction running, so a prior error was caught — future ones pause.
        self.in_error_unwind = false;
        self.capture_block_input(registers);
        self.pending_frame_name = called_decl_name(engine_state, site.instruction());

        // Only single-line spans are valid stop locations; block-wide glue spans
        // would make the UI jump to line 1.
        let span = site.span();
        let position = self.source_map.resolve_steppable(span);

        let gate = match self.read_pause_gate(engine_state, position.as_ref()) {
            Some(gate) => gate,
            // Terminate requested: this instruction must not proceed. Read first
            // because that check runs on every instruction, steppable or not.
            None => return,
        };

        // Nothing below applies without a stop location: `read_pause_gate` only
        // matches breakpoints against a position, stepping only stops where
        // there is a line to show, and the tape only records real lines.
        let Some(position) = position else { return };

        if gate.wants_locals() {
            self.sync_locals_from_stack(engine_state, stack);
        }

        // Order is load-bearing from here: locals are synced before a condition
        // or logpoint is evaluated, `current_span` is set before the tape entry
        // is built, and line tracking is updated only once any pause returns.
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

        self.capture_last_return_value(&site);
    }
}
