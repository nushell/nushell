//! eBPF commands for Nushell
//!
//! These commands allow attaching Nushell closures (compiled to eBPF) to kernel
//! probe points for tracing.

mod attach;
mod detach;
mod events;
mod helpers;
mod list;
mod trace;

pub use attach::EbpfAttach;
pub use detach::EbpfDetach;
pub use events::EbpfEvents;
pub use helpers::{BpfEmit, BpfPid, BpfUid, BpfKtime};
pub use list::EbpfList;
pub use trace::EbpfTrace;

use nu_protocol::engine::Command;

/// Get all eBPF commands
pub fn commands() -> Vec<Box<dyn Command>> {
    vec![
        Box::new(EbpfAttach),
        Box::new(EbpfDetach),
        Box::new(EbpfEvents),
        Box::new(EbpfList),
        Box::new(EbpfTrace),
        // BPF helper commands (usable in closures)
        Box::new(BpfPid),
        Box::new(BpfUid),
        Box::new(BpfKtime),
        Box::new(BpfEmit),
    ]
}
