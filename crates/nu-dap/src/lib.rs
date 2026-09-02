//! # nu-dap — a Debug Adapter Protocol server for Nushell
//!
//! `nu-dap` speaks the [Debug Adapter Protocol][dap] over a byte transport so
//! any DAP-capable editor (VS Code, Zed, Neovim, …) can debug a Nushell
//! script: breakpoints, stepping, variable inspection, watch expressions,
//! interactive prompts, and recorded time-travel.
//!
//! [dap]: https://microsoft.github.io/debug-adapter-protocol/
//!
//! ## Entry points
//!
//! - [`run_stdio`] — what `nu --dap` calls, handing over the engine it has
//!   already built. Speaks DAP over this process's stdin/stdout and does the
//!   process-level setup an adapter that owns its stdio needs: rustls provider
//!   (so `http` works), child stdin detached to `NUL`, and the script's
//!   stdout/stderr captured and forwarded as DAP `output` events so they
//!   can't corrupt the protocol stream.
//!
//! - [`serve`] — the same dispatch loop over any
//!   [`BufRead`](std::io::BufRead) + [`Write`](std::io::Write), for embedding
//!   behind another transport (socket, named pipe) or when the host wants to
//!   own its stdio. Touches no process-level stdio, so the host routes the
//!   debugged script's output itself (see `run_stdio` for reference).
//!
//! The two are independent: a host that would rather own the loop can drop to
//! `server::run_loop` with a pre-built [`dap::protocol::DapWriter`], with no
//! changes to the debugger, engine, or state layers.

// `Result<_, ShellError>` trips `clippy::result_large_err`; nushell allows this
// workspace-wide for the same reason, so mirror it here.
#![allow(clippy::result_large_err)]
#![warn(unreachable_pub)]
// Tests may `unwrap` (matches nushell); production code stays `unwrap`-free.
#![cfg_attr(test, allow(clippy::unwrap_used))]

mod debugger;
mod engine;
mod eval_scratch;
mod file_table;
mod print_cmd;
mod server;
mod source_map;
mod state;
mod stdio;
mod variables;

/// DAP wire protocol: message framing and the typed request/response/event
/// payloads. Exposed for integrators that build atop [`serve`]; most callers
/// only need [`run_stdio`] or [`serve`].
pub mod dap;

/// Run the adapter over this process's stdin/stdout — the default entry point.
///
/// Does the full process-stdio setup (see the crate docs) and blocks until the
/// DAP client disconnects.
///
/// `engine_state` is the host's fully built engine (commands, plugins,
/// environment, `$nu`), never mutated: each run clones it and adjusts only
/// what a debug session needs (engine.rs), so debugged scripts see the same
/// nushell the host would run them with.
pub fn run_stdio(engine_state: nu_protocol::engine::EngineState) {
    // Keep the DAP stream for ourselves: externals get NUL stdin, and process
    // stdout/stderr are swapped for capture pipes (see stdio.rs).
    let dap_stdin = stdio::detach_stdin();
    let capture = stdio::install_output_capture();

    // DAP frames go to the duplicated real stdout; the process-level stdout now
    // points at the capture pipe.
    let writer = dap::protocol::DapWriter::new(Box::new(
        capture
            .dap_out
            .try_clone()
            .expect("clone dap stdout handle"),
    ));
    stdio::spawn_forwarders(capture, &writer);

    server::run_loop(std::io::BufReader::new(dap_stdin), writer, engine_state);
}

/// Run the DAP server over a caller-supplied transport.
///
/// Reads framed DAP requests from `input` and writes responses/events to
/// `output`, on the calling thread, until the client disconnects. Unlike
/// [`run_stdio`], it touches no process-level stdio and installs no rustls
/// provider — the host owns that. `engine_state` is the template each run
/// clones, as for [`run_stdio`].
pub fn serve<R, W>(input: R, output: W, engine_state: nu_protocol::engine::EngineState)
where
    R: std::io::BufRead,
    W: std::io::Write + Send + 'static,
{
    let writer = dap::protocol::DapWriter::new(Box::new(output));
    server::run_loop(input, writer, engine_state);
}

#[cfg(test)]
#[macro_use]
extern crate nu_test_support;

#[cfg(test)]
use nu_test_support::harness::main;
