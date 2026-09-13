//! Read-only inspection handlers, all served from the pause snapshot:
//! threads, stackTrace, scopes, variables, evaluate.

use super::{Session, THREAD_ID};
use crate::dap::protocol::Request;
use crate::dap::types::{
    EvaluateArgs, EvaluateResponse, Scope, ScopesResponse, StackFrame, StackTraceArgs,
    StackTraceResponse, Thread, ThreadsResponse, Variable, VariablesArgs, VariablesResponse,
};
use crate::state::PauseSnapshot;

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
        // anything else runs in the scratch engine with the shadow vars in
        // scope, where the script's own commands aren't visible.
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

        let value: Result<nu_protocol::Value, String> = {
            let fast = if is_bare_name {
                self.with_state(|session| {
                    session
                        .active_shadow_vars()
                        .values()
                        .find(|sv| sv.name == bare)
                        .map(|sv| sv.value.clone())
                })
                .flatten()
            } else {
                None
            };
            match (fast, &self.state) {
                (Some(v), _) => Ok(v),
                (None, Some(state)) => {
                    let vars = {
                        let session = state.session_state.lock();
                        session
                            .active_shadow_vars()
                            .values()
                            .map(|sv| (sv.name.clone(), sv.value.clone()))
                            .collect::<Vec<_>>()
                    };
                    let mut guard = state.scratch.lock();
                    match guard.as_mut() {
                        Some(scratch) => scratch.eval(&expr, &vars),
                        None => Err("no scratch engine: the run has not started".into()),
                    }
                }
                (None, None) => Err("no active session".into()),
            }
        };

        match value {
            Ok(v) => {
                // Park it in the snapshot arena so structured results are
                // expandable in the client.
                let parts = self.with_state_mut(|session| {
                    let snap = session.active_snapshot_mut();
                    let idx = crate::variables::add_value(
                        snap,
                        "result".into(),
                        &v,
                        usize::MAX, // beyond eager horizon: lazy children
                    );
                    let node = &snap.var_arena[idx];
                    (
                        node.var.value.clone(),
                        node.var.variables_reference,
                        node.var.type_.clone(),
                    )
                });
                // No state means the eval thread cached nothing; defaults do
                // for a one-off render, and an empty label map just leaves a
                // closure as `<closure>`.
                let (result, var_ref, type_) = parts.unwrap_or_else(|| {
                    let config = nu_protocol::Config::default();
                    let cache = crate::state::RenderCache::default();
                    let ctx = crate::variables::RenderCtx {
                        config: &config,
                        cache: &cache,
                    };
                    (crate::variables::short_render(&v, ctx), 0, None)
                });
                self.writer.respond(
                    seq,
                    cmd,
                    EvaluateResponse {
                        result,
                        variables_reference: var_ref,
                        type_,
                    },
                );
            }
            Err(e) => self.writer.respond_error(seq, cmd, e),
        }
    }
}
