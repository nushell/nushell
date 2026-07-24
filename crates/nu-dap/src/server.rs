//! The DAP server: reads requests from stdin on the main thread and
//! dispatches them. Never blocks on the eval thread (see state.rs).
//!
//! Expected client sequence (VS Code):
//!   initialize -> launch -> setBreakpoints* -> configurationDone
//!   -> [threads/stackTrace/scopes/variables while paused]
//!   -> continue/next/stepIn/stepOut/pause -> ... -> disconnect

use crate::dap::protocol::{DapWriter, Request, read_message};
use crate::dap::types::*;
use crate::engine::spawn_eval_thread;
use crate::state::{DebugState, PauseSnapshot, RunMode};
use serde_json::{Value as Json, json};
use std::io::BufRead;
use std::sync::Arc;

/// nu evaluation is single-threaded from DAP's perspective.
pub(crate) const THREAD_ID: i64 = 1;

/// The transport-agnostic DAP dispatch loop: read framed requests from
/// `reader`, dispatch them, and write responses/events through `writer`.
/// Runs on the calling thread until the client disconnects. This is the
/// seam an alternative transport (socket, pipe) plugs into — see
/// [`crate::serve`]. Process-stdio capture is a separate concern owned by
/// [`crate::run_stdio`].
pub(crate) fn run_loop<R: BufRead>(mut reader: R, writer: DapWriter) {
    let mut session = Session {
        writer: writer.clone(),
        state: None,
        pending_launch: None,
        launch_args: None,
        eval_handle: None,
    };

    loop {
        match read_message(&mut reader) {
            Ok(Some(req)) => {
                let exit = session.dispatch(req);
                if exit {
                    break;
                }
            }
            Ok(None) => break, // EOF: client went away
            Err(e) => {
                writer.output("stderr", format!("nu-dap protocol error: {e}\n"));
                break;
            }
        }
    }
}

struct Session {
    writer: DapWriter,
    state: Option<Arc<DebugState>>,
    pending_launch: Option<LaunchArgs>,
    /// Retained past configurationDone so `restart` can respawn the run.
    launch_args: Option<LaunchArgs>,
    eval_handle: Option<std::thread::JoinHandle<()>>,
}

impl Session {
    /// Returns true when the session should end.
    fn dispatch(&mut self, req: Request) -> bool {
        let seq = req.seq;
        let cmd = req.command.clone();
        match cmd.as_str() {
            "initialize" => {
                // Version stamp in the debug console: lets a user verify
                // which adapter build their session is actually running.
                self.writer.output(
                    "console",
                    format!("nu-dap {} (nushell in-tree)\n", env!("CARGO_PKG_VERSION")),
                );
                self.writer.respond(
                    seq,
                    &cmd,
                    json!({
                        "supportsConfigurationDoneRequest": true,
                        "supportsTerminateRequest": true,
                        // Hot restart: re-run the (possibly edited) script from
                        // disk in the same session, keeping breakpoints.
                        "supportsRestartRequest": true,
                        "supportsConditionalBreakpoints": true,
                        "supportsLogPoints": true,
                        "supportsExceptionInfoRequest": true,
                        // Time travel: VS Code shows Step Back / Reverse
                        // Continue buttons and sends stepBack/reverseContinue.
                        "supportsStepBack": true,
                        "exceptionBreakpointFilters": [{
                            "filter": "error",
                            "label": "Runtime errors",
                            "description": "Pause when a command raises an error (including errors later caught by try/catch).",
                            "default": true,
                        }],
                        // Explicitly unsupported for v1:
                        "supportsFunctionBreakpoints": false,
                        "supportsEvaluateForHovers": true,
                    }),
                );
                self.writer.event("initialized", json!({}));
            }

            "launch" => match serde_json::from_value::<LaunchArgs>(req.arguments) {
                Ok(args) => {
                    self.state = Some(Arc::new(DebugState::new(
                        args.stop_on_entry,
                        args.time_travel.unwrap_or(true),
                        args.time_travel_max_steps.unwrap_or(10000),
                    )));
                    self.launch_args = Some(args.clone());
                    self.pending_launch = Some(args);
                    self.writer.respond(seq, &cmd, Json::Null);
                }
                Err(e) => self
                    .writer
                    .respond_error(seq, &cmd, format!("bad launch args: {e}")),
            },

            "setBreakpoints" => {
                let args: SetBreakpointsArgs = match serde_json::from_value(req.arguments) {
                    Ok(a) => a,
                    Err(e) => {
                        self.writer
                            .respond_error(seq, &cmd, format!("bad args: {e}"));
                        return false;
                    }
                };
                let path = args.source.path.as_deref().map(crate::paths::canonical_str);

                let mut verified = Vec::new();
                if let (Some(state), Some(path)) = (&self.state, path) {
                    let mut inner = state.inner.lock().expect("state poisoned");
                    let mut map = std::collections::BTreeMap::new();
                    for bp in &args.breakpoints {
                        let id = inner.next_bp_id;
                        inner.next_bp_id += 1;
                        // Snap to the next line with instructions (no-op with
                        // optimistic verification before parsing finishes —
                        // the eval thread reconciles + re-announces then).
                        let (snapped, ok) = inner.snap_line(&path, bp.line);
                        let line = if map.contains_key(&snapped) {
                            bp.line
                        } else {
                            snapped
                        };
                        map.insert(
                            line,
                            crate::state::BpProps {
                                id,
                                verified: ok,
                                condition: bp.condition.clone(),
                                log_message: bp.log_message.clone(),
                            },
                        );
                        verified.push(Breakpoint {
                            id: Some(id),
                            verified: ok,
                            line,
                            source: Some(args.source.clone()),
                        });
                    }
                    inner.breakpoints.insert(path.clone(), map);
                }
                self.writer
                    .respond(seq, &cmd, json!({ "breakpoints": verified }));
            }

            "setExceptionBreakpoints" => {
                let filters: Vec<String> = req
                    .arguments
                    .get("filters")
                    .and_then(|f| serde_json::from_value(f.clone()).ok())
                    .unwrap_or_default();
                if let Some(state) = &self.state {
                    let mut inner = state.inner.lock().expect("state poisoned");
                    inner.break_on_error = filters.iter().any(|f| f == "error");
                }
                self.writer.respond(seq, &cmd, json!({ "breakpoints": [] }));
            }

            "exceptionInfo" => {
                let info = self
                    .with_state(|inner| inner.exception_info.clone())
                    .flatten();
                match info {
                    Some((id, description)) => self.writer.respond(
                        seq,
                        &cmd,
                        json!({
                            "exceptionId": id,
                            "description": description,
                            "breakMode": "always",
                        }),
                    ),
                    None => self
                        .writer
                        .respond_error(seq, &cmd, "not paused on an exception"),
                }
            }

            "configurationDone" => {
                self.writer.respond(seq, &cmd, Json::Null);
                if let (Some(launch), Some(state)) =
                    (self.pending_launch.take(), self.state.clone())
                {
                    self.eval_handle = Some(spawn_eval_thread(launch, state, self.writer.clone()));
                }
            }

            "threads" => {
                self.writer.respond(
                    seq,
                    &cmd,
                    json!({ "threads": [{ "id": THREAD_ID, "name": "nu script" }] }),
                );
            }

            "stackTrace" => {
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
                    &cmd,
                    json!({ "stackFrames": frames, "totalFrames": total }),
                );
            }

            "scopes" => {
                self.writer.respond(
                    seq,
                    &cmd,
                    json!({
                        "scopes": [
                            Scope { name: "Locals".into(),
                                    variables_reference: PauseSnapshot::LOCALS_REF,
                                    expensive: false },
                            Scope { name: "Pipeline".into(),
                                    variables_reference: PauseSnapshot::PIPELINE_REF,
                                    expensive: false },
                            // Nushell special variables ($nu, $env) as records.
                            Scope { name: "Globals".into(),
                                    variables_reference: PauseSnapshot::GLOBALS_REF,
                                    expensive: true },
                            // Raw IR registers; collapsed by default.
                            Scope { name: "Registers".into(),
                                    variables_reference: PauseSnapshot::REGISTERS_REF,
                                    expensive: true },
                            // Rolling stdout/stderr tails of externals.
                            Scope { name: "Process".into(),
                                    variables_reference: PauseSnapshot::PROCESS_REF,
                                    expensive: true },
                        ]
                    }),
                );
            }

            "variables" => {
                let args: VariablesArgs = match serde_json::from_value(req.arguments) {
                    Ok(a) => a,
                    Err(e) => {
                        self.writer
                            .respond_error(seq, &cmd, format!("bad args: {e}"));
                        return false;
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
                self.writer.respond(seq, &cmd, json!({ "variables": vars }));
            }

            "evaluate" => {
                // Fast path: a bare `$name` serves straight from the shadow
                // snapshot (hover stays cheap). Anything else runs in the
                // scratch engine with the shadow variables in scope.
                // Limitation (documented): custom commands from the debugged
                // script are not visible to the scratch engine.
                let args: EvaluateArgs = match serde_json::from_value(req.arguments) {
                    Ok(a) => a,
                    Err(e) => {
                        self.writer
                            .respond_error(seq, &cmd, format!("bad args: {e}"));
                        return false;
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
                                let inner = state.inner.lock().expect("state poisoned");
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
                        let (result, var_ref, type_) =
                            parts.unwrap_or((crate::variables::short_render(&v), 0, None));
                        self.writer.respond(
                            seq,
                            &cmd,
                            json!({
                                "result": result,
                                "variablesReference": var_ref,
                                "type": type_,
                            }),
                        );
                    }
                    Err(e) => self.writer.respond_error(seq, &cmd, e),
                }
            }

            // Backward navigation over the recorded tape — never resumes the
            // eval thread; just moves the cursor and re-serves history.
            "stepBack" => {
                let target = self.with_state(|inner| {
                    inner
                        .view_index
                        .or_else(|| inner.frontier())
                        .map(|c| c.saturating_sub(1))
                });
                if let Some(Some(t)) = target {
                    self.tt_goto(Some(t), "step");
                }
                self.writer.respond(seq, &cmd, Json::Null);
            }
            "reverseContinue" => {
                let target = self
                    .with_state(|inner| {
                        let cur = inner.view_index.or_else(|| inner.frontier())?;
                        let prev_bp = (0..cur).rev().find(|&j| {
                            inner
                                .timeline
                                .get(j)
                                .map(|e| e.is_breakpoint)
                                .unwrap_or(false)
                        });
                        Some(prev_bp.unwrap_or(0))
                    })
                    .flatten();
                if let Some(t) = target {
                    let reason = self
                        .with_state(|i| i.timeline.get(t).map(|e| e.is_breakpoint).unwrap_or(false))
                        .unwrap_or(false);
                    self.tt_goto(Some(t), if reason { "breakpoint" } else { "step" });
                }
                self.writer
                    .respond(seq, &cmd, json!({ "allThreadsContinued": true }));
            }

            "continue" => {
                // In the past: replay forward to the next recorded breakpoint,
                // else return to the live frontier. Only at the frontier do we
                // resume real execution.
                if let Some(Some(cur)) = self.with_state(|i| i.view_index) {
                    let target = self
                        .with_state(|inner| {
                            let front = inner.frontier().unwrap_or(cur);
                            (cur + 1..front).find(|&j| {
                                inner
                                    .timeline
                                    .get(j)
                                    .map(|e| e.is_breakpoint)
                                    .unwrap_or(false)
                            })
                        })
                        .flatten();
                    match target {
                        Some(t) => self.tt_goto(Some(t), "breakpoint"),
                        None => self.tt_goto(None, "step"), // back at the live frontier
                    }
                } else {
                    self.resume(RunMode::Continue);
                }
                self.writer
                    .respond(seq, &cmd, json!({ "allThreadsContinued": true }));
            }
            "next" | "stepIn" => {
                if let Some(Some(cur)) = self.with_state(|i| i.view_index) {
                    // Forward one line through recorded history.
                    let front = self.with_state(|i| i.frontier()).flatten().unwrap_or(cur);
                    if cur + 1 >= front {
                        self.tt_goto(None, "step"); // reached the live frontier
                    } else {
                        self.tt_goto(Some(cur + 1), "step");
                    }
                } else {
                    let mode = if cmd == "next" {
                        self.with_state(|inner| RunMode::StepOver {
                            depth: inner.paused_depth,
                            line: inner.paused_line,
                        })
                    } else {
                        self.with_state(|inner| RunMode::StepIn {
                            depth: inner.paused_depth,
                            line: inner.paused_line,
                        })
                    }
                    .unwrap_or(RunMode::Continue);
                    self.resume(mode);
                }
                self.writer.respond(seq, &cmd, Json::Null);
            }
            "stepOut" => {
                if let Some(Some(cur)) = self.with_state(|i| i.view_index) {
                    let target = self.with_state(|inner| {
                        let front = inner.frontier().unwrap_or(cur);
                        let cur_depth = inner.timeline.get(cur).map(|e| e.depth).unwrap_or(0);
                        let j = (cur + 1..=front).find(|&j| {
                            inner
                                .timeline
                                .get(j)
                                .map(|e| e.depth < cur_depth)
                                .unwrap_or(false)
                        });
                        (j, front)
                    });
                    match target {
                        Some((Some(j), front)) if j < front => self.tt_goto(Some(j), "step"),
                        _ => self.tt_goto(None, "step"),
                    }
                } else {
                    let mode = self
                        .with_state(|inner| RunMode::StepOut {
                            depth: inner.paused_depth,
                        })
                        .unwrap_or(RunMode::Continue);
                    self.resume(mode);
                }
                self.writer.respond(seq, &cmd, Json::Null);
            }
            "pause" => {
                if let Some(state) = &self.state {
                    let mut inner = state.inner.lock().expect("state poisoned");
                    inner.run_mode = RunMode::PauseNow;
                }
                self.writer.respond(seq, &cmd, Json::Null);
            }

            // Custom request from the extension's "Visualize" action: return
            // the complete value behind a variable as JSON (the Variables
            // tree itself is depth/width-bounded; this is not). Expandable
            // nodes are addressed by variablesReference; leaves (strings,
            // binaries) by containerReference + name.
            "nuDapVisualize" => {
                let args: VisualizeArgs = match serde_json::from_value(req.arguments) {
                    Ok(a) => a,
                    Err(e) => {
                        self.writer
                            .respond_error(seq, &cmd, format!("bad args: {e}"));
                        return false;
                    }
                };
                let found = self
                    .with_state(|inner| {
                        let snap = inner.active_snapshot();
                        let node = match (args.variables_reference, args.container_reference) {
                            (Some(r), _) if r > 0 => snap
                                .var_arena
                                .iter()
                                .find(|n| n.var.variables_reference == r),
                            (_, Some(cr)) => snap.var_refs.get(&cr).and_then(|children| {
                                children
                                    .iter()
                                    .map(|&i| &snap.var_arena[i])
                                    .find(|n| Some(&n.var.name) == args.name.as_ref())
                            }),
                            _ => None,
                        };
                        node.map(|n| {
                            let mut truncated = false;
                            let json = crate::variables::to_json(&n.value, 0, &mut truncated);
                            (json, n.value.get_type().to_string(), truncated)
                        })
                    })
                    .flatten();
                match found {
                    Some((value, type_, truncated)) => self.writer.respond(
                        seq,
                        &cmd,
                        json!({ "value": value, "type": type_, "truncated": truncated }),
                    ),
                    None => self.writer.respond_error(
                        seq,
                        &cmd,
                        "no value for that reference (stale after resume?)",
                    ),
                }
            }

            // Answer to a `nu-dap-ui` prompt event: hand it to the eval
            // thread blocked inside the input shim.
            "nuDapUiReply" => {
                let id = req.arguments.get("id").and_then(|v| v.as_u64());
                if let (Some(id), Some(state)) = (id, &self.state) {
                    let reply =
                        crate::state::UiReply {
                            cancelled: req
                                .arguments
                                .get("cancelled")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false),
                            index: req
                                .arguments
                                .get("index")
                                .and_then(|v| v.as_u64())
                                .map(|v| v as usize),
                            indices: req.arguments.get("indices").and_then(|v| v.as_array()).map(
                                |a| {
                                    a.iter()
                                        .filter_map(|x| x.as_u64())
                                        .map(|x| x as usize)
                                        .collect()
                                },
                            ),
                            value: req
                                .arguments
                                .get("value")
                                .and_then(|v| v.as_str())
                                .map(str::to_string),
                        };
                    state
                        .ui
                        .replies
                        .lock()
                        .expect("ui bridge poisoned")
                        .insert(id, reply);
                    state.ui.cv.notify_all();
                }
                self.writer.respond(seq, &cmd, Json::Null);
            }

            "restart" => {
                // Hot restart: tear down the current run quietly and respawn
                // from the stored launch args. The script file is re-read from
                // disk, so edits made while debugging take effect. Breakpoints
                // carry over; VS Code re-sends them if the file changed.
                match (self.launch_args.clone(), self.state.take()) {
                    (Some(args), Some(old_state)) => {
                        old_state.request_restart_teardown();
                        // Not joined: the old thread unwinds on its own via the
                        // interrupt signal. Joining here could block the DAP
                        // loop behind a long-running native command.
                        let _ = self.eval_handle.take();

                        let new_state = Arc::new(DebugState::new(
                            args.stop_on_entry,
                            args.time_travel.unwrap_or(true),
                            args.time_travel_max_steps.unwrap_or(10000),
                        ));
                        {
                            let old = old_state.inner.lock().expect("state poisoned");
                            let mut new = new_state.inner.lock().expect("state poisoned");
                            new.breakpoints = old.breakpoints.clone();
                        }
                        self.state = Some(new_state.clone());
                        self.writer.respond(seq, &cmd, Json::Null);
                        self.eval_handle =
                            Some(spawn_eval_thread(args, new_state, self.writer.clone()));
                    }
                    (args, state) => {
                        // Restore whatever we had; nothing to restart yet.
                        self.state = state;
                        let _ = args;
                        self.writer.respond_error(
                            seq,
                            &cmd,
                            "nothing to restart: no active launch",
                        );
                    }
                }
            }

            "terminate" | "disconnect" => {
                if let Some(state) = &self.state {
                    state.request_terminate();
                }
                self.writer.respond(seq, &cmd, Json::Null);
                if cmd == "disconnect" {
                    return true;
                }
            }

            // Politely acknowledge anything we don't implement.
            other => {
                self.writer.respond(seq, other, Json::Null);
            }
        }
        false
    }

    fn resume(&self, mode: RunMode) {
        if let Some(state) = &self.state {
            state.resume(mode);
        }
    }

    /// Move the time-travel cursor and emit `stopped`. `target = Some(i)`
    /// views `timeline[i]` (rebuilds `history_snapshot`); `None` returns to
    /// the live frontier. Never resumes the eval thread.
    fn tt_goto(&self, target: Option<usize>, reason: &'static str) {
        let Some(state) = &self.state else { return };
        {
            let mut inner = state.inner.lock().expect("state poisoned");
            inner.view_index = target;
            if let Some(i) = target {
                let entry = inner.timeline.get(i).cloned();
                let baseline = inner.baseline_env.clone();
                let nu = inner.nu_constant.clone();
                if let Some(entry) = entry {
                    inner.history_snapshot = crate::variables::build_history_snapshot(
                        &entry,
                        baseline.as_ref(),
                        nu.as_ref(),
                    );
                }
            }
        }
        self.writer.event(
            "stopped",
            json!({
                "reason": reason,
                "threadId": THREAD_ID,
                "allThreadsStopped": true,
            }),
        );
    }

    fn with_state<T>(&self, f: impl FnOnce(&crate::state::Inner) -> T) -> Option<T> {
        self.state.as_ref().map(|s| {
            let inner = s.inner.lock().expect("state poisoned");
            f(&inner)
        })
    }

    fn with_state_mut<T>(&self, f: impl FnOnce(&mut crate::state::Inner) -> T) -> Option<T> {
        self.state.as_ref().map(|s| {
            let mut inner = s.inner.lock().expect("state poisoned");
            f(&mut inner)
        })
    }
}
