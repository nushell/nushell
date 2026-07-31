//! Read-only inspection handlers, all served from the pause snapshot:
//! threads, stackTrace, scopes, variables, evaluate.

use super::{Session, THREAD_ID};
use crate::dap::protocol::Request;
use crate::dap::types::{EvaluateArgs, Scope, StackFrame, StackTraceArgs, Variable, VariablesArgs};
use crate::state::PauseSnapshot;
use serde_json::json;

impl Session {
    pub(super) fn on_threads(&mut self, seq: i64, cmd: &str) {
        self.writer.respond(
            seq,
            cmd,
            json!({ "threads": [{ "id": THREAD_ID, "name": "nu script" }] }),
        );
    }

    pub(super) fn on_stack_trace(&mut self, seq: i64, cmd: &str, req: Request) {
        let _args: StackTraceArgs =
            serde_json::from_value(req.arguments).unwrap_or(StackTraceArgs {
                thread_id: THREAD_ID,
                start_frame: None,
                levels: None,
            });
        let frames: Vec<StackFrame> = self
            .with_state(|inner| inner.active_snapshot().frames.clone())
            .unwrap_or_default();
        let total = frames.len();
        self.writer.respond(
            seq,
            cmd,
            json!({ "stackFrames": frames, "totalFrames": total }),
        );
    }

    pub(super) fn on_scopes(&mut self, seq: i64, cmd: &str) {
        // Locals and Globals always show; the situational scopes (Pipeline,
        // Registers, Process) appear only when they have content, so the panel
        // isn't cluttered with empty sections at most stops.
        let (pipeline, registers, process) = self
            .with_state(|inner| {
                let snap = inner.active_snapshot();
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

        self.writer.respond(seq, cmd, json!({ "scopes": scopes }));
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
            .with_state_mut(|inner| {
                let snap = inner.active_snapshot_mut();
                // Lazy hydration: materialize this node's children on
                // first expansion.
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
        self.writer.respond(seq, cmd, json!({ "variables": vars }));
    }

    pub(super) fn on_evaluate(&mut self, seq: i64, cmd: &str, req: Request) {
        // Fast path: a bare `$name` serves straight from the snapshot (cheap
        // hover); anything else runs in the scratch engine with the shadow vars
        // in scope. Limitation: the script's own commands aren't visible there.
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
                self.with_state(|inner| {
                    inner
                        .shadow_vars
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
                        let inner = state.session_state.lock().expect("state poisoned");
                        inner
                            .shadow_vars
                            .values()
                            .map(|sv| (sv.name.clone(), sv.value.clone()))
                            .collect::<Vec<_>>()
                    };
                    let mut guard = state.scratch.lock().expect("scratch poisoned");
                    guard
                        .get_or_insert_with(crate::eval_scratch::Scratch::new)
                        .eval(&expr, &vars)
                }
                (None, None) => Err("no active session".into()),
            }
        };

        match value {
            Ok(v) => {
                // Park the result in the snapshot arena so structured
                // results are expandable in the client.
                let parts = self.with_state_mut(|inner| {
                    let snap = inner.active_snapshot_mut();
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
                // No state means no cached config either; defaults will do for
                // a value we are only rendering once.
                let (result, var_ref, type_) = parts.unwrap_or_else(|| {
                    (
                        crate::variables::short_render(&v, &nu_protocol::Config::default()),
                        0,
                        None,
                    )
                });
                self.writer.respond(
                    seq,
                    cmd,
                    json!({
                        "result": result,
                        "variablesReference": var_ref,
                        "type": type_,
                    }),
                );
            }
            Err(e) => self.writer.respond_error(seq, cmd, e),
        }
    }
}
