//! Session lifecycle handlers: initialize, launch, configurationDone,
//! restart, and terminate/disconnect.

use super::Session;
use crate::dap::protocol::Request;
use crate::dap::types::LaunchArgs;
use crate::engine::spawn_eval_thread;
use crate::state::DebugState;
use serde_json::{Value as Json, json};
use std::sync::Arc;

impl Session {
    pub(super) fn on_initialize(&mut self, seq: i64, cmd: &str) {
        // Version stamp in the debug console: lets a user verify
        // which adapter build their session is actually running.
        self.writer.output(
            "console",
            format!("nu-dap {} (nushell in-tree)\n", env!("CARGO_PKG_VERSION")),
        );
        self.writer.respond(
            seq,
            cmd,
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

    pub(super) fn on_launch(&mut self, seq: i64, cmd: &str, req: Request) {
        match serde_json::from_value::<LaunchArgs>(req.arguments) {
            Ok(args) => {
                self.state = Some(Arc::new(DebugState::new(
                    args.stop_on_entry,
                    args.time_travel.unwrap_or(true),
                    args.time_travel_max_steps.unwrap_or(10000),
                )));
                self.launch_args = Some(args.clone());
                self.pending_launch = Some(args);
                self.writer.respond(seq, cmd, Json::Null);
            }
            Err(e) => self
                .writer
                .respond_error(seq, cmd, format!("bad launch args: {e}")),
        }
    }

    pub(super) fn on_configuration_done(&mut self, seq: i64, cmd: &str) {
        self.writer.respond(seq, cmd, Json::Null);
        if let (Some(launch), Some(state)) = (self.pending_launch.take(), self.state.clone()) {
            self.eval_handle = Some(spawn_eval_thread(launch, state, self.writer.clone()));
        }
    }

    pub(super) fn on_restart(&mut self, seq: i64, cmd: &str) {
        // Hot restart: tear down the run quietly and respawn from the stored
        // launch args. The script is re-read from disk (edits take effect);
        // breakpoints carry over.
        match (self.launch_args.clone(), self.state.take()) {
            (Some(args), Some(old_state)) => {
                old_state.request_restart_teardown();
                // Not joined: the old thread unwinds itself via the interrupt
                // signal; joining could block the DAP loop behind a native call.
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
                self.writer.respond(seq, cmd, Json::Null);
                self.eval_handle = Some(spawn_eval_thread(args, new_state, self.writer.clone()));
            }
            (args, state) => {
                // Restore whatever we had; nothing to restart yet.
                self.state = state;
                let _ = args;
                self.writer
                    .respond_error(seq, cmd, "nothing to restart: no active launch");
            }
        }
    }

    pub(super) fn on_terminate_or_disconnect(&mut self, seq: i64, cmd: &str) -> bool {
        if let Some(state) = &self.state {
            state.request_terminate();
        }
        self.writer.respond(seq, cmd, Json::Null);
        cmd == "disconnect"
    }
}
