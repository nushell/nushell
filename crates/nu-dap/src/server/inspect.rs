//! Read-only inspection handlers, all served from the pause snapshot:
//! threads, stackTrace, scopes, variables, evaluate.

use super::{Session, THREAD_ID};
use crate::dap::protocol::{DapWriter, Request};
use crate::dap::types::{
    EvaluateArgs, EvaluateResponse, Scope, ScopesResponse, StackFrame, StackTraceArgs,
    StackTraceResponse, Thread, ThreadsResponse, Variable, VariablesArgs, VariablesResponse,
};
use crate::state::{DebugState, PauseSnapshot};
use nu_protocol::Value;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::RecvTimeoutError;

impl Session {
    pub(super) fn on_threads(&mut self, seq: i64, cmd: &str) {
        self.writer.respond(
            seq,
            cmd,
            ThreadsResponse {
                threads: vec![Thread {
                    id: THREAD_ID,
                    name: "nu script",
                }],
            },
        );
    }

    pub(super) fn on_stack_trace(&mut self, seq: i64, cmd: &str, req: Request) {
        let _args: StackTraceArgs =
            serde_json::from_value(req.arguments).unwrap_or(StackTraceArgs {
                thread_id: THREAD_ID,
                start_frame: None,
                levels: None,
            });
        // Frames are stored 1-based (and reused by the time-travel tape), so
        // the client's numbering is applied here, on the way out, rather than
        // baked into what is recorded.
        let frames: Vec<StackFrame> = self
            .with_state(|session| session.active_snapshot().frames.clone())
            .unwrap_or_default()
            .into_iter()
            .map(|mut frame| {
                frame.line = self.coords.line_to_client(frame.line);
                frame.column = self.coords.column_to_client(frame.column);
                frame
            })
            .collect();
        let total = frames.len();
        self.writer.respond(
            seq,
            cmd,
            StackTraceResponse {
                stack_frames: frames,
                total_frames: total,
            },
        );
    }

    pub(super) fn on_scopes(&mut self, seq: i64, cmd: &str) {
        // Locals and Globals always show; Pipeline, Registers and Process
        // only when they have content, to keep the panel free of empty
        // sections.
        let (pipeline, registers, process) = self
            .with_state(|session| {
                let snap = session.active_snapshot();
                let filled = |r: i64| snap.var_refs.get(&r).is_some_and(|c| !c.is_empty());
                (
                    filled(PauseSnapshot::PIPELINE_REF),
                    filled(PauseSnapshot::REGISTERS_REF),
                    filled(PauseSnapshot::PROCESS_REF),
                )
            })
            .unwrap_or((false, false, false));

        let mut scopes = vec![Scope {
            name: "Locals".into(),
            variables_reference: PauseSnapshot::LOCALS_REF,
            expensive: false,
        }];
        if pipeline {
            scopes.push(Scope {
                name: "Pipeline".into(),
                variables_reference: PauseSnapshot::PIPELINE_REF,
                expensive: false,
            });
        }
        // Nushell special variables ($nu, $env) as records.
        scopes.push(Scope {
            name: "Globals".into(),
            variables_reference: PauseSnapshot::GLOBALS_REF,
            expensive: true,
        });
        if registers {
            // Raw IR registers; collapsed by default.
            scopes.push(Scope {
                name: "Registers".into(),
                variables_reference: PauseSnapshot::REGISTERS_REF,
                expensive: true,
            });
        }
        if process {
            // Rolling stdout/stderr tails of externals.
            scopes.push(Scope {
                name: "Process".into(),
                variables_reference: PauseSnapshot::PROCESS_REF,
                expensive: true,
            });
        }

        self.writer.respond(seq, cmd, ScopesResponse { scopes });
    }

    pub(super) fn on_variables(&mut self, seq: i64, cmd: &str, req: Request) {
        let args: VariablesArgs = match serde_json::from_value(req.arguments) {
            Ok(a) => a,
            Err(e) => {
                self.writer
                    .respond_error(seq, cmd, format!("bad args: {e}"));
                return;
            }
        };
        let vars: Vec<Variable> = self
            .with_state_mut(|session| {
                let snap = session.active_snapshot_mut();
                // Lazy hydration: children appear on first expansion.
                crate::variables::materialize_children(snap, args.variables_reference);
                snap.var_refs
                    .get(&args.variables_reference)
                    .map(|children| {
                        children
                            .iter()
                            .map(|&i| snap.var_arena[i].var.clone())
                            .collect()
                    })
                    .unwrap_or_default()
            })
            .unwrap_or_default();
        self.writer
            .respond(seq, cmd, VariablesResponse { variables: vars });
    }

    pub(super) fn on_evaluate(&mut self, seq: i64, cmd: &str, req: Request) {
        // A bare `$name` is served straight from the snapshot (cheap hover);
        // anything else goes to the scratch engine, off this thread.
        let args: EvaluateArgs = match serde_json::from_value(req.arguments) {
            Ok(a) => a,
            Err(e) => {
                self.writer
                    .respond_error(seq, cmd, format!("bad args: {e}"));
                return;
            }
        };

        let expr = args.expression.trim().to_string();
        let bare = expr.strip_prefix('$').unwrap_or(&expr);
        let is_bare_name =
            !bare.is_empty() && bare.chars().all(|c| c.is_alphanumeric() || c == '_');

        if is_bare_name
            && let Some(state) = &self.state
            && let Some(v) = state
                .session_state
                .lock()
                .active_shadow_vars()
                .values()
                .find(|sv| sv.name == bare)
                .map(|sv| sv.value.clone())
        {
            respond_with_value(&self.writer, state, seq, cmd, v);
            return;
        }

        match self.state.clone() {
            Some(state) => spawn_evaluate(seq, cmd.to_string(), expr, state, self.writer.clone()),
            None => self.writer.respond_error(seq, cmd, "no active session"),
        }
    }
}

/// How long an `evaluate` waits before giving up on its expression and
/// unwinding it.
const EVALUATE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// Evaluate `expr` in the scratch engine and answer from there.
///
/// Off the server thread, which reads stdin: a slow expression must not stop
/// the adapter from seeing `continue` or `disconnect`. Responding out of order
/// is fine — DAP matches a response to its request by `request_seq`.
fn spawn_evaluate(seq: i64, cmd: String, expr: String, state: Arc<DebugState>, writer: DapWriter) {
    std::thread::spawn(move || {
        let vars = {
            let session = state.session_state.lock();
            session
                .active_shadow_vars()
                .values()
                .map(|sv| (sv.name.clone(), sv.value.clone()))
                .collect::<Vec<_>>()
        };

        // The expression's own flag: raised on timeout, it aborts this
        // evaluation and only this one, even if it is still queued behind a
        // logpoint for the engine.
        let interrupt = Arc::new(AtomicBool::new(false));
        let (tx, rx) = std::sync::mpsc::channel();
        let worker = std::thread::Builder::new()
            .stack_size(crate::engine::EVAL_STACK_SIZE)
            .spawn({
                let (state, interrupt) = (state.clone(), interrupt.clone());
                move || {
                    let _ = tx.send(state.scratch_eval(&expr, &vars, &interrupt));
                }
            });
        if worker.is_err() {
            writer.respond_error(seq, &cmd, "could not start the evaluation thread");
            return;
        }

        match rx.recv_timeout(EVALUATE_TIMEOUT) {
            Ok(Ok(v)) => respond_with_value(&writer, &state, seq, &cmd, v),
            Ok(Err(e)) => writer.respond_error(seq, &cmd, e),
            Err(RecvTimeoutError::Timeout) => {
                interrupt.store(true, Ordering::SeqCst);
                writer.respond_error(
                    seq,
                    &cmd,
                    format!(
                        "expression did not finish within {}s and was cancelled",
                        EVALUATE_TIMEOUT.as_secs()
                    ),
                );
            }
            Err(RecvTimeoutError::Disconnected) => {
                writer.respond_error(seq, &cmd, "evaluation failed: the engine panicked")
            }
        }
    });
}

/// Park the value in the pause snapshot's arena, so a structured result is
/// expandable in the client, and respond.
fn respond_with_value(writer: &DapWriter, state: &DebugState, seq: i64, cmd: &str, v: Value) {
    let mut session = state.session_state.lock();
    let snap = session.active_snapshot_mut();
    let idx = crate::variables::add_value(
        snap,
        "result".into(),
        &v,
        usize::MAX, // beyond eager horizon: lazy children
    );
    let node = &snap.var_arena[idx];
    writer.respond(
        seq,
        cmd,
        EvaluateResponse {
            result: node.var.value.clone(),
            variables_reference: node.var.variables_reference,
            type_: node.var.type_.clone(),
        },
    );
}
