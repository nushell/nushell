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
mod stepping;

use crate::dap::protocol::DapWriter;
use crate::source_map::SourceMap;
use crate::state::{DebugState, RunMode};
use nu_protocol::ast::Block;
use nu_protocol::debugger::Debugger;
use nu_protocol::engine::{EngineState, Stack};
use nu_protocol::ir::{Instruction, IrBlock};
use nu_protocol::{PipelineData, PipelineExecutionData, ShellError, Span};
use serde_json::json;
use std::sync::Arc;

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
    in_value: Option<nu_protocol::Value>,
}

pub(crate) struct DapDebugger {
    state: Arc<DebugState>,
    writer: DapWriter,
    source_map: SourceMap,
    /// Call-ish stack maintained from enter/leave_block. Depth 0 is the
    /// top-level script block.
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
    last_result: Option<nu_protocol::Value>,
    /// Block depth at the previous steppable instruction — lets time-travel
    /// recording fire on depth changes (entering/leaving closures), matching
    /// forward step-into granularity.
    last_depth: Option<usize>,
}

impl std::fmt::Debug for DapDebugger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DapDebugger")
            .field("frames", &self.frames.len())
            .finish()
    }
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

    /// Current shadow variables as (name, value) pairs for scratch eval.
    fn shadow_vars_for_eval(&self) -> Vec<(String, nu_protocol::Value)> {
        let inner = self.state.inner.lock().expect("debug state poisoned");
        inner
            .shadow_vars
            .values()
            .map(|sv| (sv.name.clone(), sv.value.clone()))
            .collect()
    }

    fn scratch_eval(&self, expr: &str) -> Result<nu_protocol::Value, String> {
        let vars = self.shadow_vars_for_eval();
        let mut guard = self.state.scratch.lock().expect("scratch poisoned");
        guard
            .get_or_insert_with(crate::eval_scratch::Scratch::new)
            .eval(expr, &vars)
    }

    fn scratch_interpolate(&self, template: &str) -> String {
        let vars = self.shadow_vars_for_eval();
        let mut guard = self.state.scratch.lock().expect("scratch poisoned");
        crate::eval_scratch::interpolate(
            guard.get_or_insert_with(crate::eval_scratch::Scratch::new),
            template,
            &vars,
        )
    }

    /// The pause loop: publish snapshot, emit `stopped`, block until resumed.
    fn pause(
        &mut self,
        engine_state: &EngineState,
        reason: &'static str,
        ir_block: &IrBlock,
        instruction_index: usize,
        registers: &[PipelineExecutionData],
        description: Option<&str>,
    ) {
        // Sync the output capture so the Process scope tails are current.
        crate::stdio::flush_output(std::time::Duration::from_millis(300));

        // IR listing for the extension's "Show IR" panel. Custom DAP event;
        // clients that don't know it simply ignore it.
        self.writer.event(
            "nuDapIr",
            json!({
                "text": format!("{}", ir_block.display(engine_state)),
                "instructionIndex": instruction_index,
                "instructionCount": ir_block.instructions.len(),
            }),
        );

        let mut inner = self.state.inner.lock().expect("debug state poisoned");
        if reason != "exception" {
            inner.exception_info = None;
        }
        self.build_snapshot(
            engine_state,
            &mut inner,
            ir_block,
            instruction_index,
            registers,
        );
        inner.paused_line = self
            .current_span
            .and_then(|s| self.source_map.resolve(s))
            .map(|p| p.line)
            .unwrap_or(0);
        inner.paused_depth = self.depth();
        inner.paused = true;
        inner.resume_requested = false;
        self.state.paused_cv.notify_all();

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

        while !inner.resume_requested {
            inner = self
                .state
                .resume_cv
                .wait(inner)
                .expect("debug state poisoned");
        }
        inner.paused = false;
        inner.resume_requested = false;

        if inner.terminate_requested {
            // Raise nu's interrupt signal: evaluation unwinds with
            // ShellError::Interrupted at the next signal check.
            engine_state.signals().trigger();
        }
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
        // Real evaluation stack (#18708): source of truth for this frame's
        // locals/env, read at each steppable line by `sync_locals_from_stack`.
        stack: &Stack,
        ir_block: &IrBlock,
        instruction_index: usize,
        registers: &[PipelineExecutionData],
    ) {
        self.source_map.refresh(engine_state);
        // New instruction running, so a prior error was caught — future ones pause.
        self.in_error_unwind = false;

        // First instruction of a fresh block: register 0 is `$in` (per element
        // for each/where closures). It's register-based, not on the Stack, so
        // stash it on the frame; `sync_locals_from_stack` injects it later.
        if self.just_entered_block {
            self.just_entered_block = false;
            if let Some(PipelineData::Value(v, _)) = registers.first().map(|r| &r.body)
                && !matches!(v, nu_protocol::Value::Nothing { .. })
                && let Some(frame) = self.frames.last_mut()
            {
                frame.in_value = Some(v.clone());
            }
        }

        // Frame naming only: a `Call` to a named decl labels the block its
        // `enter_block` will push; anything else clears it so a builtin call
        // (no block follows) can't mislabel a later block.
        match &ir_block.instructions[instruction_index] {
            Instruction::Call { decl_id, .. } => {
                self.pending_frame_name = Some(engine_state.get_decl(*decl_id).name().to_string());
            }
            _ => {
                self.pending_frame_name = None;
            }
        }

        // Only single-line spans are valid stop locations; block-wide glue spans
        // would make the UI jump to line 1.
        let span = ir_block.spans[instruction_index];
        let pos = self.source_map.resolve_steppable(span);

        // One lock: terminate check, breakpoint lookup, run mode, time-travel flag.
        // Condition/logpoint eval happens after, off-lock (scratch has its own).
        let (bp_props, run_mode, time_travel) = {
            let inner = self.state.inner.lock().expect("debug state poisoned");
            if inner.terminate_requested {
                engine_state.signals().trigger();
                return;
            }
            let props = pos.as_ref().and_then(|p| {
                // Only on first arrival at the line, not per instruction.
                if self.last_line == Some(p.line) {
                    return None;
                }
                inner
                    .breakpoints
                    .get(&p.path)
                    .and_then(|m| m.get(&(p.line as i64)))
                    .cloned()
            });
            (props, inner.run_mode, inner.time_travel)
        };

        // Snapshot locals+env from the Stack whenever we might pause, eval a
        // condition/logpoint, or record. Skipped on the plain-`continue` fast
        // path so hot loops don't clone per line.
        if pos.is_some()
            && (bp_props.is_some() || !matches!(run_mode, RunMode::Continue) || time_travel)
        {
            self.sync_locals_from_stack(engine_state, stack);
        }

        let mut reason: Option<&'static str> = None;
        let mut note: Option<String> = None;
        if let Some(props) = bp_props {
            if let Some(template) = &props.log_message {
                // Logpoint: emit, never pause.
                let msg = self.scratch_interpolate(template);
                self.writer.output("console", format!("{msg}\n"));
            } else if let Some(cond) = &props.condition {
                match self.scratch_eval(cond) {
                    Ok(nu_protocol::Value::Bool { val: true, .. }) => {
                        reason = Some("breakpoint");
                    }
                    Ok(nu_protocol::Value::Bool { val: false, .. }) => {}
                    Ok(v) => {
                        // Pause on a broken condition rather than skip the breakpoint.
                        reason = Some("breakpoint");
                        note = Some(format!(
                            "condition `{cond}` returned {} (expected bool) — pausing",
                            crate::variables::short_render(&v)
                        ));
                    }
                    Err(e) => {
                        reason = Some("breakpoint");
                        note = Some(format!("condition `{cond}` failed: {e} — pausing"));
                    }
                }
            } else {
                reason = Some("breakpoint");
            }
        }

        if reason.is_none() && pos.is_some() {
            let is_call = matches!(
                &ir_block.instructions[instruction_index],
                Instruction::Call { .. }
            );
            reason = self.should_pause_mode(pos.as_ref(), run_mode, is_call);
        }

        if pos.is_some() {
            self.current_span = Some(span);
        }

        // Record on the tape at the same granularity forward stepping stops
        // (line/depth change or a call boundary), so Step Back reaches every
        // point F11 would — pipe stages and same-line closure bodies included.
        if let Some(p) = &pos {
            let depth = self.depth();
            let line_changed = self.last_line != Some(p.line);
            let depth_changed = self.last_depth != Some(depth);
            let is_call = matches!(
                &ir_block.instructions[instruction_index],
                Instruction::Call { .. }
            );
            // At a call boundary, capture the value flowing in so the past
            // view can show `in → cmd`.
            let pipe_input = match &ir_block.instructions[instruction_index] {
                Instruction::Call {
                    decl_id, src_dst, ..
                } => registers
                    .get(src_dst.get() as usize)
                    .and_then(|r| match &r.body {
                        PipelineData::Value(v, _)
                            if !matches!(v, nu_protocol::Value::Nothing { .. }) =>
                        {
                            Some((
                                engine_state.get_decl(*decl_id).name().to_string(),
                                v.clone(),
                            ))
                        }
                        other @ (PipelineData::ListStream(..) | PipelineData::ByteStream(..)) => {
                            Some((
                                engine_state.get_decl(*decl_id).name().to_string(),
                                nu_protocol::Value::string(
                                    crate::variables::describe_stream(other),
                                    Span::unknown(),
                                ),
                            ))
                        }
                        _ => None,
                    }),
                _ => None,
            };
            let frames = self.build_frames();
            let last_result = self.last_result.clone();
            let mut inner = self.state.inner.lock().expect("debug state poisoned");
            let granular = line_changed || depth_changed || is_call;
            if (inner.time_travel && granular) || reason.is_some() {
                let entry = crate::state::TimelineEntry {
                    frames,
                    shadow_vars: inner.shadow_vars.clone(),
                    env_shadow: inner.env_shadow.clone(),
                    last_result,
                    pipe_input,
                    depth,
                    is_breakpoint: reason == Some("breakpoint"),
                };
                inner.push_timeline(entry);
                inner.view_index = None; // execution advanced: back at the frontier
            }
        }

        if let Some(r) = reason {
            if let Some(n) = &note {
                self.writer.output("console", format!("nu-dap: {n}\n"));
            }
            self.pause(
                engine_state,
                r,
                ir_block,
                instruction_index,
                registers,
                None,
            );
        }

        // Remember this line so later instructions on it don't refire the
        // breakpoint. Non-steppable instructions never touch line tracking.
        if let Some(p) = &pos {
            self.last_line = Some(p.line);
            self.last_depth = Some(self.depth());
            if let Some(frame) = self.frames.last_mut() {
                frame.last_line = Some(p.line);
            }
        }
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
        // Latest result for the Locals `return` entry: a Call writes to src_dst,
        // a Return's output is in src. Streams are described, never drained.
        if error.is_none() {
            let reg = match &ir_block.instructions[instruction_index] {
                Instruction::Call { src_dst, .. } => Some(src_dst.get() as usize),
                Instruction::Return { src } => Some(src.get() as usize),
                _ => None,
            };
            if let Some(idx) = reg
                && let Some(r) = registers.get(idx)
            {
                self.last_result = Some(match &r.body {
                    PipelineData::Value(v, _) => v.clone(),
                    PipelineData::Empty => nu_protocol::Value::nothing(Span::unknown()),
                    other => nu_protocol::Value::string(
                        crate::variables::describe_stream(other),
                        Span::unknown(),
                    ),
                });
            }
            return;
        }
        let err = error.expect("error present");
        // An error unwinds through every enclosing call instruction; pause
        // only at the innermost (first) report.
        if self.in_error_unwind {
            return;
        }
        let wanted = {
            let inner = self.state.inner.lock().expect("debug state poisoned");
            inner.break_on_error && !inner.terminate_requested
        };
        if !wanted {
            return;
        }
        self.in_error_unwind = true;

        // Point the top frame at the failing instruction, even when its span
        // is multi-line (better an approximate position than none).
        let span = ir_block.spans[instruction_index];
        if self.source_map.resolve(span).is_some() {
            self.current_span = Some(span);
        }

        let mut description = format!("{err}");
        let exception_id = {
            // Variant name from the Debug form, e.g. "GenericError { .. }".
            let dbg = format!("{err:?}");
            dbg.split([' ', '(', '{'])
                .next()
                .unwrap_or("ShellError")
                .to_string()
        };
        // "External command had a non-zero exit code" says nothing — the
        // command's actual complaint went to stderr. Attach its tail.
        if exception_id == "NonZeroExitCode" {
            crate::stdio::flush_output(std::time::Duration::from_millis(500));
            let tail = crate::stdio::recent_output("stderr");
            let tail = tail.trim();
            if !tail.is_empty() {
                // Keep the dialog readable: last ~1000 chars.
                let start = tail.len().saturating_sub(1000);
                let mut cut = start;
                while !tail.is_char_boundary(cut) {
                    cut += 1;
                }
                description = format!("{description}\n\n{}", &tail[cut..]);
            }
        }
        // This path skips the enter_instruction sync, so refresh from the Stack.
        self.sync_locals_from_stack(engine_state, stack);
        {
            let mut inner = self.state.inner.lock().expect("debug state poisoned");
            inner.exception_info = Some((exception_id, description.clone()));
        }
        self.pause(
            engine_state,
            "exception",
            ir_block,
            instruction_index,
            registers,
            Some(&description),
        );
    }
}
