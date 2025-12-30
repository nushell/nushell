//! eBPF support for Nushell
//!
//! This crate provides the ability to compile a subset of Nushell closures
//! to eBPF bytecode and attach them to kernel probe points for tracing.
//!
//! # Platform Support
//!
//! This crate only works on Linux. On other platforms, the commands will
//! return an error indicating that eBPF is not supported.
//!
//! # Requirements
//!
//! - Linux kernel 4.18+ (for BPF CO-RE support)
//! - CAP_BPF capability or root access
//! - Kernel compiled with CONFIG_BPF=y

use nu_protocol::engine::{EngineState, StateWorkingSet};

#[cfg(target_os = "linux")]
pub mod compiler;
#[cfg(target_os = "linux")]
pub mod loader;

pub mod commands;

#[cfg(target_os = "linux")]
pub use compiler::EbpfProgram;

/// Add eBPF commands to the engine state
pub fn add_ebpf_context(mut engine_state: EngineState) -> EngineState {
    let delta = {
        let mut working_set = StateWorkingSet::new(&engine_state);

        for cmd in commands::commands() {
            working_set.add_decl(cmd);
        }

        working_set.render()
    };

    if let Err(err) = engine_state.merge_delta(delta) {
        eprintln!("Error creating eBPF command context: {err:?}");
    }

    engine_state
}
