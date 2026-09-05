//! Session lifecycle handlers: initialize, launch, configurationDone,
//! restart, and terminate/disconnect.

use super::Session;
use crate::dap::protocol::Request;
use crate::dap::types::{
    Capabilities, DapEvent, ExceptionBreakpointFilter, InitializeArgs, LaunchArgs,
};
use crate::engine::spawn_eval_thread;
use crate::state::ClientCoords;
use crate::state::DebugState;
use std::sync::Arc;

impl Session {
    pub(super) fn on_initialize(&mut self, seq: i64, cmd: &str, req: Request) {
        // The client states how it numbers lines and columns here. Malformed
        // arguments fall back to the spec's default (1-based) rather than
        // failing the handshake over a field almost nobody sends.
        let args: InitializeArgs = serde_json::from_value(req.arguments).unwrap_or_default();
        self.coords = ClientCoords::new(args.lines_start_at1, args.columns_start_at1);

        // Version stamp in the debug console, so a user can tell which
        // adapter build they are running.
        self.writer.output(
            "console",
            format!("nu-dap {} (nushell in-tree)\n", env!("CARGO_PKG_VERSION")),
        );
        self.writer.respond(
            seq,
            cmd,
            Capabilities {
                supports_configuration_done_request: true,
                supports_terminate_request: true,
                supports_restart_request: true,
                supports_conditional_breakpoints: true,
                supports_log_points: true,
                supports_exception_info_request: true,
                supports_step_back: true,
                supports_evaluate_for_hovers: true,
                supports_function_breakpoints: false,
                supports_breakpoint_locations_request: true,
                exception_breakpoint_filters: vec![ExceptionBreakpointFilter {
                    filter: "error",
                    label: "Runtime errors",
                    description: "Pause when a command raises an error (including errors later caught by try/catch).",
                    default: true,
                }],
            },
        );
        self.writer.event(DapEvent::Initialized);
    }

    pub(super) fn on_launch(&mut self, seq: i64, cmd: &str, req: Request) {
        match serde_json::from_value::<LaunchArgs>(req.arguments) {
            Ok(args) => {
                self.state = Some(Arc::new(DebugState::new(
                    args.stop_on_entry,
                    args.time_travel.unwrap_or(true),
                    args.time_travel_max_steps.unwrap_or(10000),
                    self.files.clone(),
                    self.coords,
                )));
                self.launch_args = Some(args.clone());
                self.pending_launch = Some(args);
                self.writer.respond(seq, cmd, ());
            }
            Err(e) => self
                .writer
                .respond_error(seq, cmd, format!("bad launch args: {e}")),
        }
    }

    pub(super) fn on_configuration_done(&mut self, seq: i64, cmd: &str) {
        self.writer.respond(seq, cmd, ());
        if let (Some(launch), Some(state)) = (self.pending_launch.take(), self.state.clone()) {
            self.eval_handle = Some(spawn_eval_thread(
                launch,
                state,
                self.writer.clone(),
                self.engine_state.clone(),
            ));
        }
    }

    pub(super) fn on_restart(&mut self, seq: i64, cmd: &str) {
        // Hot restart: tear down quietly and respawn from the stored launch
        // args. The script is re-read from disk; breakpoints carry over.
        match (self.launch_args.clone(), self.state.take()) {
            (Some(args), Some(old_state)) => {
                old_state.request_restart_teardown();
                // Not joined: the old thread unwinds itself on the interrupt
                // signal, and joining could block the DAP loop behind a
                // native call.
                let _ = self.eval_handle.take();

                // Same `FileTable` as the outgoing run, so the breakpoints
                // copied below stay keyed to their files.
                let new_state = Arc::new(DebugState::new(
                    args.stop_on_entry,
                    args.time_travel.unwrap_or(true),
                    args.time_travel_max_steps.unwrap_or(10000),
                    self.files.clone(),
                    self.coords,
                ));
                {
                    let old = old_state.session_state.lock();
                    let mut new = new_state.session_state.lock();
                    new.breakpoints = old.breakpoints.clone();
                    // A restart stays inside one DAP session, so the client is
                    // never asked to replay its configuration. The id counter
                    // has to come along or the next `setBreakpoints` reissues
                    // ids the breakpoints copied above still hold, and the
                    // exception filter has to come along or a restart quietly
                    // switches pausing-on-errors back on.
                    new.next_bp_id = old.next_bp_id;
                    new.break_on_error = old.break_on_error;
                }
                self.state = Some(new_state.clone());
                self.writer.respond(seq, cmd, ());
                // From the pristine template, so decls parsed in the previous
                // run don't leak into this one.
                self.eval_handle = Some(spawn_eval_thread(
                    args,
                    new_state,
                    self.writer.clone(),
                    self.engine_state.clone(),
                ));
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
        self.writer.respond(seq, cmd, ());
        cmd == "disconnect"
    }
}
