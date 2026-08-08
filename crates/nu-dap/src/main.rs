//! `nu-dap` binary: a Debug Adapter Protocol server for Nushell scripts,
//! speaking DAP over stdio. Launched by the VS Code extension (see
//! ../extension) — and the shape a `nu --dap` entry point would take.
//!
//! All logic lives in the library crate ([`nu_dap`]); this is just the default
//! stdio entry point. See `nu_dap`'s crate docs for the public API and the
//! transport-agnostic [`nu_dap::serve`] seam.

fn main() {
    nu_dap::run_stdio();
}
