mod imp;
mod options;

use std::sync::OnceLock;

pub use options::*;

pub struct ExperimentalOption {
    value: OnceLock<bool>,
    marker: &'static (dyn imp::DynExperimentalOptionMarker + Send + Sync),
}

/// Where an experimental option sits in its life-cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stability {
    /// Likely to change, disabled by default.
    Unstable,
    /// Final API, disabled by default.
    Stable,
    /// Final API, enabled by default.
    StableDefault,
    /// Deprecated, will be removed and prints a warning.
    Deprecated,
}
