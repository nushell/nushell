//! eBPF commands for Nushell
//!
//! These commands allow attaching Nushell closures (compiled to eBPF) to kernel
//! probe points for tracing.

mod attach;
mod counters;
mod detach;
mod events;
mod helpers;
mod list;
mod trace;

pub use attach::EbpfAttach;
pub use counters::EbpfCounters;
pub use detach::EbpfDetach;
pub use events::EbpfEvents;
pub use helpers::{BpfComm, BpfCount, BpfEmit, BpfEmitComm, BpfKtime, BpfPid, BpfUid};
pub use list::EbpfList;
pub use trace::EbpfTrace;

use nu_protocol::engine::Command;

/// Get all eBPF commands
pub fn commands() -> Vec<Box<dyn Command>> {
    vec![
        Box::new(EbpfAttach),
        Box::new(EbpfCounters),
        Box::new(EbpfDetach),
        Box::new(EbpfEvents),
        Box::new(EbpfList),
        Box::new(EbpfTrace),
        // BPF helper commands (usable in closures)
        Box::new(BpfPid),
        Box::new(BpfUid),
        Box::new(BpfKtime),
        Box::new(BpfComm),
        Box::new(BpfCount),
        Box::new(BpfEmit),
        Box::new(BpfEmitComm),
    ]
}
