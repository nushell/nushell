//! The DAP server: reads requests from stdin on the main thread and
//! dispatches them. Never blocks on the eval thread (see state.rs).
//!
//! Expected client sequence (VS Code):
//!   initialize -> launch -> setBreakpoints* -> configurationDone
//!   -> [threads/stackTrace/scopes/variables while paused]
//!   -> continue/next/stepIn/stepOut/pause -> ... -> disconnect
//!
//! `dispatch` is a thin router; each request's handler lives in a submodule,
//! grouped by concern (lifecycle, breakpoints, inspect, stepping, timetravel,
//! custom), as extra `impl Session` blocks.

mod breakpoints;
mod custom;
mod inspect;
mod lifecycle;
mod stepping;
mod timetravel;

use crate::dap::protocol::{DapWriter, Request, read_message};
use crate::dap::types::LaunchArgs;
use crate::state::{DebugState, RunMode};
use std::io::BufRead;
use std::sync::Arc;

/// nu evaluation is single-threaded from DAP's perspective.
pub(crate) const THREAD_ID: i64 = 1;

/// The transport-agnostic DAP dispatch loop: read framed requests from
/// `reader`, dispatch them, write responses/events through `writer`, on the
/// calling thread until the client disconnects. The seam an alternative
/// transport plugs into (see [`crate::serve`]); process-stdio capture belongs
/// to [`crate::run_stdio`].
///
/// `engine_state` is the template each run is cloned from (see
/// [`crate::engine::spawn_eval_thread`]).
pub(crate) fn run_loop<R: BufRead>(
    mut reader: R,
    writer: DapWriter,
    engine_state: nu_protocol::engine::EngineState,
) {
    let mut session = Session {
        writer: writer.clone(),
        engine_state,
        state: None,
        files: crate::file_table::FileTable::default(),
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
    /// The host's engine, untouched: every run gets its own clone, so parsing
    /// or restarting never mutates what the next run starts from.
    engine_state: nu_protocol::engine::EngineState,
    state: Option<Arc<DebugState>>,
    /// Session-wide path <-> `FileId` interning, handed to every `DebugState`.
    /// Lives here so ids survive a `restart` and carried-over breakpoints stay
    /// pointed at their files.
    files: crate::file_table::FileTable,
    pending_launch: Option<LaunchArgs>,
    /// Retained past configurationDone so `restart` can respawn the run.
    launch_args: Option<LaunchArgs>,
    eval_handle: Option<std::thread::JoinHandle<()>>,
}

impl Session {
    /// Route a request to its handler. Returns true when the session should end.
    fn dispatch(&mut self, req: Request) -> bool {
        let seq = req.seq;
        let cmd = req.command.clone();
        match cmd.as_str() {
            "initialize" => self.on_initialize(seq, &cmd),
            "launch" => self.on_launch(seq, &cmd, req),
            "setBreakpoints" => self.on_set_breakpoints(seq, &cmd, req),
            "setExceptionBreakpoints" => self.on_set_exception_breakpoints(seq, &cmd, req),
            "exceptionInfo" => self.on_exception_info(seq, &cmd),
            "configurationDone" => self.on_configuration_done(seq, &cmd),
            "threads" => self.on_threads(seq, &cmd),
            "stackTrace" => self.on_stack_trace(seq, &cmd, req),
            "scopes" => self.on_scopes(seq, &cmd),
            "variables" => self.on_variables(seq, &cmd, req),
            "evaluate" => self.on_evaluate(seq, &cmd, req),
            "stepBack" => self.on_step_back(seq, &cmd),
            "reverseContinue" => self.on_reverse_continue(seq, &cmd),
            "continue" => self.on_continue(seq, &cmd),
            "next" | "stepIn" => self.on_next_or_step_in(seq, &cmd),
            "stepOut" => self.on_step_out(seq, &cmd),
            "pause" => self.on_pause(seq, &cmd),
            "nuDapVisualize" => self.on_visualize(seq, &cmd, req),
            "nuDapUiReply" => self.on_ui_reply(seq, &cmd, req),
            "restart" => self.on_restart(seq, &cmd),
            "terminate" | "disconnect" => return self.on_terminate_or_disconnect(seq, &cmd),
            // Unimplemented requests get an error response, not a cheerful
            // `success: true` that a client would read as "it worked".
            other => {
                self.writer.respond_error(
                    seq,
                    other,
                    format!("`{other}` is not supported by nu-dap"),
                );
            }
        }
        false
    }

    fn resume(&self, mode: RunMode) {
        if let Some(state) = &self.state {
            state.resume(mode);
        }
    }

    fn with_state<T>(&self, f: impl FnOnce(&crate::state::SessionState) -> T) -> Option<T> {
        self.state.as_ref().map(|s| {
            let session = s.session_state.lock();
            f(&session)
        })
    }

    fn with_state_mut<T>(&self, f: impl FnOnce(&mut crate::state::SessionState) -> T) -> Option<T> {
        self.state.as_ref().map(|s| {
            let mut session = s.session_state.lock();
            f(&mut session)
        })
    }
}
