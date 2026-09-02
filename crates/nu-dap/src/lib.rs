//! # nu-dap — a Debug Adapter Protocol server for Nushell
//!
//! `nu-dap` speaks the [Debug Adapter Protocol][dap] over a byte transport so
//! any DAP-capable editor (VS Code, Zed, Neovim, …) can debug a Nushell
//! script: breakpoints, stepping, variable inspection, watch expressions,
//! interactive prompts, and recorded time-travel.
//!
//! [dap]: https://microsoft.github.io/debug-adapter-protocol/
//!
//! ## Public API
//!
//! There are two entry points, in order of increasing control:
//!
//! - [`run_stdio`] — the default, and what `nu --dap` calls, handing over the
//!   engine it has already built. Speaks DAP over this process's
//!   stdin/stdout and does the process-level setup an adapter that owns its
//!   stdio needs: installs the rustls provider so `http` works, detaches child
//!   stdin to `NUL`, and captures the script's stdout/stderr to forward as DAP
//!   `output` events so it can't corrupt the protocol stream.
//!
//! - [`serve`] — the transport-agnostic core: run the DAP dispatch loop over
//!   any [`BufRead`](std::io::BufRead) + [`Write`](std::io::Write). Use this to
//!   embed the adapter behind a different transport (a socket, a named pipe) or
//!   when the host process wants to own its own stdio. It does **not** touch
//!   process-level stdio, so the host is responsible for routing the debugged
//!   script's stdout/stderr if it cares (see the `stdio` handling `run_stdio`
//!   does for reference).
//!
//! ## Design note (for integrators / upstreaming)
//!
//! The stdio loop deliberately lives *in this crate* ([`run_stdio`]), so
//! `nu --dap`-style usage is a one-call affair. But the protocol loop itself
//! ([`serve`] → `server::run_loop`) is fully decoupled from the transport and
//! from process-stdio ownership. If a host would rather own the loop, it can
//! call [`serve`] with its own reader/writer, or drop to `server::run_loop`
//! with a pre-built [`dap::protocol::DapWriter`] — no changes to the debugger,
//! engine, or state layers are required.

// `ShellError` is a large enum, so any `Result<_, ShellError>` trips
// `clippy::result_large_err`. Nushell allows this workspace-wide for the same
// reason (its command trait returns exactly that); mirror it here.
#![allow(clippy::result_large_err)]
#![warn(unreachable_pub)]
// Allow `unwrap()` in tests (idiomatic, matches nushell); production code
// stays `unwrap`-free under `clippy::unwrap_used`.
#![cfg_attr(test, allow(clippy::unwrap_used))]

mod debugger;
mod engine;
mod eval_scratch;
mod paths;
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
/// Performs the full process-stdio setup (see the crate docs): rustls
/// provider, child-stdin detach, and script-output capture. Blocks until the
/// DAP client disconnects.
///
/// `engine_state` is the host's fully built engine (commands, plugins,
/// environment, `$nu`). The adapter never mutates it: each debug run clones it
/// and adjusts only what a debug session needs (engine.rs), so debugged
/// scripts see the same nushell the host would run them with.
pub fn run_stdio(engine_state: nu_protocol::engine::EngineState) {
    // Install the rustls crypto provider, or every `http` command fails with
    // "tls crypto provider not found". A no-op if the host already did it
    // (it's a `OnceLock`), which matters for hosts built without rustls-tls.
    nu_command::tls::CRYPTO_PROVIDER.default();

    // Keep the DAP stream for ourselves: externals get NUL stdin, and process
    // stdout/stderr are swapped for capture pipes so no script (or child
    // process) output can corrupt the protocol (see stdio.rs).
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
/// [`run_stdio`], this touches no process-level stdio and installs no rustls
/// provider — the host owns that. Ideal for embedding behind a socket/pipe or
/// inside a larger process. `engine_state` is the template each run clones,
/// exactly as for [`run_stdio`].
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
