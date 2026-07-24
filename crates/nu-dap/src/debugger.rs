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

use crate::dap::protocol::DapWriter;
use crate::dap::types::{Source, StackFrame};
use crate::source_map::SourceMap;
use crate::state::{DebugState, PauseSnapshot, RunMode, ShadowVar};
use crate::variables::add_value;
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
    /// Name to give the next block we enter, set when the previous
    /// instruction was a `Call` to a named decl (e.g. a custom command).
    /// Consumed by `enter_block`; cleared at the top of every
    /// `enter_instruction` so a builtin call (which enters no block) can't
    /// mislabel an unrelated later block.
    pending_frame_name: Option<String>,
    /// Source line of the previously executed instruction in the current
    /// frame (mirror of frames.last().last_line, kept for the pre-frame
    /// window and step math). A single source line compiles to several IR
    /// instructions, so breakpoints fire only when the line *changes*.
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

/// Resolve a variable's source name. Nushell stores no name on a `VarId`, but
/// the variable's declaration span points at the identifier in source. Trims
/// the leading `$` and any type annotation (`x: int` → `x`).
fn var_name(engine_state: &EngineState, var_id: nu_protocol::VarId) -> String {
    let var = engine_state.get_var(var_id);
    let bytes = engine_state.get_span_contents(var.declaration_span);
    let s = String::from_utf8_lossy(bytes);
    // `$x` / `x: int` / `--verbose` / `--tag: string` → `x` / `verbose` / `tag`.
    let s = s.trim().trim_start_matches('$');
    let s = s.split([':', ' ', '\t']).next().unwrap_or(s);
    let s = s.trim_start_matches('-');
    if s.is_empty() {
        format!("var{}", var_id.get())
    } else {
        s.to_string()
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

    /// Snapshot the current frame's locals and environment from the real
    /// evaluation `Stack` (nushell #18708) into shared state. Called at every
    /// steppable line we might pause on, evaluate a condition/logpoint at, or
    /// record for time-travel — so the pause snapshot, scratch eval, and the
    /// tape all read genuine values (params, closure captures, mutations)
    /// rather than the old IR reconstruction. `$in` is the one exception:
    /// register-based, injected from the frame's captured value.
    fn sync_locals_from_stack(&self, engine_state: &EngineState, stack: &Stack) {
        let mut vars: std::collections::HashMap<usize, ShadowVar> =
            std::collections::HashMap::new();
        for (var_id, value) in &stack.vars {
            // Nushell's reserved specials: $nu/$env live in Globals, $in is
            // injected below from register 0.
            if *var_id == nu_protocol::NU_VARIABLE_ID
                || *var_id == nu_protocol::ENV_VARIABLE_ID
                || *var_id == nu_protocol::IN_VARIABLE_ID
            {
                continue;
            }
            vars.insert(
                var_id.get(),
                ShadowVar {
                    name: var_name(engine_state, *var_id),
                    value: value.clone(),
                },
            );
        }
        if let Some(v) = self.frames.last().and_then(|f| f.in_value.clone()) {
            vars.insert(
                nu_protocol::IN_VARIABLE_ID.get(),
                ShadowVar {
                    name: "in".to_string(),
                    value: v,
                },
            );
        }
        // Full runtime env (engine baseline + this stack's overlays/mutations).
        let env = stack.get_env_vars(engine_state);
        let mut inner = self.state.inner.lock().expect("debug state poisoned");
        inner.shadow_vars = vars;
        inner.env_shadow = env;
    }

    /// Decide whether the current run mode wants to pause at this position
    /// (breakpoints are handled separately in `enter_instruction`).
    /// `is_call` marks call instructions — pipe-stage boundaries — where
    /// step-into also stops, so F11 walks a builtin pipeline stage by stage.
    fn should_pause_mode(
        &self,
        pos: Option<&crate::source_map::SourcePos>,
        run_mode: RunMode,
        is_call: bool,
    ) -> Option<&'static str> {
        match run_mode {
            RunMode::Continue => None,
            RunMode::PauseNow => Some("entry"),
            RunMode::StepIn { depth, line } => match pos {
                Some(p) if p.line != line || self.depth() != depth || is_call => Some("step"),
                _ => None,
            },
            RunMode::StepOver { depth, line } => match pos {
                Some(_) if self.depth() < depth => Some("step"),
                Some(p) if self.depth() == depth && p.line != line => Some("step"),
                _ => None,
            },
            RunMode::StepOut { depth } => {
                if self.depth() < depth {
                    Some("step")
                } else {
                    None
                }
            }
        }
    }

    /// Stack frames, innermost first (DAP order), resolved to file/line.
    /// Shared by the live snapshot and time-travel recording.
    fn build_frames(&self) -> Vec<StackFrame> {
        let mut frames = Vec::new();
        for (i, frame) in self.frames.iter().rev().enumerate() {
            // Innermost frame reports the current instruction position;
            // callers report their call site.
            let span = if i == 0 {
                self.current_span.or(frame.at).or(frame.span)
            } else {
                frame.at.or(frame.span)
            };
            let pos = span.and_then(|s| self.source_map.resolve(s));
            frames.push(StackFrame {
                id: i as i64,
                name: frame.name.clone(),
                source: pos.as_ref().map(|p| Source {
                    name: std::path::Path::new(&p.path)
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string()),
                    path: Some(p.path.clone()),
                }),
                line: pos.as_ref().map(|p| p.line as i64).unwrap_or(0),
                column: pos.as_ref().map(|p| p.column as i64).unwrap_or(0),
            });
        }
        frames
    }

    /// Build the pause snapshot: stack frames + Locals/Pipeline/Globals/…
    fn build_snapshot(
        &self,
        engine_state: &EngineState,
        inner: &mut crate::state::Inner,
        ir_block: &IrBlock,
        instruction_index: usize,
        registers: &[PipelineExecutionData],
    ) {
        let mut snap = PauseSnapshot::new();
        snap.frames = self.build_frames();

        // Cache $nu + baseline env once, so the server can rebuild historical
        // Globals for time-travel without touching engine_state.
        if inner.nu_constant.is_none() {
            inner.nu_constant = engine_state
                .get_constant(nu_protocol::NU_VARIABLE_ID)
                .cloned();
            inner.baseline_env = Some(
                engine_state
                    .render_env_vars()
                    .into_iter()
                    .map(|(k, v)| (k.to_string(), v.clone()))
                    .collect(),
            );
        }

        // Locals scope. `return` (the most recent expression/command result)
        // first, then shadow vars sorted by name for a stable UI.
        let mut locals_children = Vec::new();
        if let Some(v) = &self.last_result {
            locals_children.push(add_value(&mut snap, "return".to_string(), v, 0));
        }
        let mut vars: Vec<&ShadowVar> = inner.shadow_vars.values().collect();
        vars.sort_by(|a, b| a.name.cmp(&b.name));
        for sv in vars {
            locals_children.push(add_value(&mut snap, sv.name.clone(), &sv.value, 0));
        }
        snap.var_refs
            .insert(PauseSnapshot::LOCALS_REF, locals_children);

        // Pipeline scope: semantic view only. Paused at a call instruction —
        // a pipe-stage boundary — it shows the value about to flow INTO that
        // command: nu's `$in` for the next stage. Raw registers live in
        // their own collapsed scope below.
        let mut pipeline_children = Vec::new();
        if let Instruction::Call {
            decl_id, src_dst, ..
        } = &ir_block.instructions[instruction_index]
            && let Some(reg) = registers.get(src_dst.get() as usize)
        {
            let name = format!("in → {}", engine_state.get_decl(*decl_id).name());
            match &reg.body {
                PipelineData::Value(v, _) if !matches!(v, nu_protocol::Value::Nothing { .. }) => {
                    pipeline_children.push(add_value(&mut snap, name, v, 0));
                }
                other @ (PipelineData::ListStream(..) | PipelineData::ByteStream(..)) => {
                    // Streams can't be inspected without draining them,
                    // but kind/origin/size are known without reading.
                    let v = nu_protocol::Value::string(
                        crate::variables::describe_stream(other),
                        Span::unknown(),
                    );
                    pipeline_children.push(add_value(&mut snap, name, &v, 0));
                }
                _ => {}
            }
        }
        snap.var_refs
            .insert(PauseSnapshot::PIPELINE_REF, pipeline_children);

        // Registers scope: the evaluator's raw working slots, for reading
        // alongside the IR panel.
        let mut register_children = Vec::new();
        for (i, reg) in registers.iter().enumerate() {
            if let PipelineData::Value(v, _) = &reg.body
                && !matches!(v, nu_protocol::Value::Nothing { .. })
            {
                register_children.push(add_value(&mut snap, format!("%{i}"), v, 0));
            }
        }
        snap.var_refs
            .insert(PauseSnapshot::REGISTERS_REF, register_children);

        // Process scope: rolling tails of what externals/drains wrote to the
        // captured process stdout/stderr.
        let mut process_children = Vec::new();
        for (label, category) in [("last output", "stdout"), ("last error output", "stderr")] {
            let tail = crate::stdio::recent_output(category);
            if !tail.is_empty() {
                let v = nu_protocol::Value::string(tail, Span::unknown());
                process_children.push(add_value(&mut snap, label.to_string(), &v, 0));
            }
        }
        snap.var_refs
            .insert(PauseSnapshot::PROCESS_REF, process_children);

        // Globals scope: nushell's special variables as expandable records.
        // `$nu` (config/paths/pid/os-info) and `$env` — the latter is the full
        // runtime env snapshotted from the Stack (`env_shadow`), as the record
        // scripts actually reference.
        let mut globals_children = Vec::new();
        if let Some(nu) = engine_state.get_constant(nu_protocol::NU_VARIABLE_ID) {
            let v = nu.clone();
            globals_children.push(add_value(&mut snap, "$nu".to_string(), &v, 0));
        }
        {
            let env_map: std::collections::BTreeMap<&String, &nu_protocol::Value> =
                inner.env_shadow.iter().collect();
            let mut rec = nu_protocol::Record::new();
            for (k, v) in env_map {
                rec.push(k.clone(), v.clone());
            }
            let env_val = nu_protocol::Value::record(rec, Span::unknown());
            globals_children.push(add_value(&mut snap, "$env".to_string(), &env_val, 0));
        }
        snap.var_refs
            .insert(PauseSnapshot::GLOBALS_REF, globals_children);

        inner.snapshot = snap;
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
            "nu-dap-ir",
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
        // Prefer the name of the decl we're about to enter (custom command),
        // captured from the preceding `Call` instruction. Otherwise fall back
        // to the block's `file.nu:line`.
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
        // Fresh frame: no line executed in it yet.
        self.last_line = None;

        self.just_entered_block = true;
        // Params and locals for this block now come straight from the real
        // Stack at each pause (see `sync_locals_from_stack`) — no pre-binding.
    }

    fn leave_block(&mut self, _engine_state: &EngineState, _block: &Block) {
        self.frames.pop();
        // Back in the caller: restore its line tracking so returning onto
        // the call line doesn't re-fire breakpoints there.
        self.last_line = self.frames.last().and_then(|f| f.last_line);
    }

    fn enter_instruction(
        &mut self,
        engine_state: &EngineState,
        // Real evaluation stack (nushell #18708): the source of truth for this
        // frame's locals and environment. Read at each steppable line by
        // `sync_locals_from_stack`.
        stack: &Stack,
        ir_block: &IrBlock,
        instruction_index: usize,
        registers: &[PipelineExecutionData],
    ) {
        self.source_map.refresh(engine_state);
        // A new instruction is executing, so any previous error was handled
        // (caught by try/catch) — future errors are fresh pauses again.
        self.in_error_unwind = false;

        // First instruction of a freshly-entered block: register 0 holds the
        // block's pipeline input — that's `$in` (per element for closures
        // driven by each/where). `$in` is register-based, not on the Stack, so
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

        // Frame naming only: a `Call` to a named decl labels the block the
        // callee's `enter_block` is about to push; any other instruction
        // clears it so a builtin call (no block follows) can't mislabel a
        // later block. Parameter/local values are no longer reconstructed
        // here — they come from the real Stack at pause time.
        match &ir_block.instructions[instruction_index] {
            Instruction::Call { decl_id, .. } => {
                self.pending_frame_name = Some(engine_state.get_decl(*decl_id).name().to_string());
            }
            _ => {
                self.pending_frame_name = None;
            }
        }

        // Only instructions whose span sits on a single source line are valid
        // stop locations. Structural glue (drain / load-empty / return) carries
        // the whole enclosing block's span; pausing there or letting it advance
        // line tracking made the UI jump to the block's first line.
        let span = ir_block.spans[instruction_index];
        let pos = self.source_map.resolve_steppable(span);

        // Single lock: terminate check, breakpoint-at-this-line lookup, the
        // current run mode, and whether time-travel recording is on.
        // Condition/logpoint evaluation happens after the lock is dropped
        // (the scratch engine has its own).
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

        // Snapshot this frame's real locals + env from the Stack for any line
        // we might pause on, evaluate a condition/logpoint at, or record on the
        // time-travel tape. Skipped on the fast path (plain `continue`, no
        // breakpoint here, time-travel off) so hot loops don't clone per line.
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
                        // Pausing on a broken condition beats silently
                        // running past the breakpoint.
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

        // Time-travel: record on the tape at the SAME granularity forward
        // stepping stops (line change, depth change, or a call / pipe-stage
        // boundary) so Step Back can reach every point F11 would — including
        // pipeline stages and closure bodies on the same source line.
        if let Some(p) = &pos {
            let depth = self.depth();
            let line_changed = self.last_line != Some(p.line);
            let depth_changed = self.last_depth != Some(depth);
            let is_call = matches!(
                &ir_block.instructions[instruction_index],
                Instruction::Call { .. }
            );
            // At a call boundary, capture the value flowing into the command
            // ($in for the next stage) so the past view can show `in → cmd`.
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

        // Remember the line we just processed so the next instruction on the
        // same source line doesn't re-trigger a breakpoint. Non-steppable
        // instructions never touch line tracking.
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
        // Capture the "latest expression result" for the Locals `return`
        // entry: a completed Call writes its result to src_dst; a Return
        // block-output is in src. Streams are described, never drained.
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
        // Refresh locals/env from the failing frame's Stack so the exception
        // snapshot shows real values (this path skips the enter_instruction sync).
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
