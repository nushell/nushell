#![allow(
    private_interfaces,
    reason = "The marker structs don't need to be exposed, only the static values."
)]

use crate::*;

mod example;

pub(crate) trait ExperimentalOptionMarker {
    const IDENTIFIER: &'static str;
    const DESCRIPTION: &'static str;
    const STABILITY: Stability;
}

pub use example::EXAMPLE;

// Include all experimental option statics in here.
// This will test them and add them to the parsing list.
pub static ALL: &'static [&ExperimentalOption] = &[&EXAMPLE];
