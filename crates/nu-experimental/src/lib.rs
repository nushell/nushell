mod imp;
mod options;

use std::sync::OnceLock;

pub use options::*;

/// Experimental option (aka feature flag).
///
/// This struct holds one experimental option that can change some part of Nushell's behavior.
/// These options let users opt in or out of experimental changes while keeping the rest stable.
/// They're useful for testing new ideas and giving users a way to go back to older behavior if needed.
///
/// You can find all options in the statics of [`nu_experimental`](crate).
/// Everything there, except [`ALL`], is a toggleable option.
/// `ALL` gives a full list and can be used to check which options are set.
/// 
/// The [`Debug`](std::fmt::Debug) implementation shows the option's identifier, stability, and 
/// current value.
/// To also include the description in the output, use the 
/// [plus sign](std::fmt::Formatter::sign_plus), e.g. `format!("{OPTION:+#?}")`.
pub struct ExperimentalOption {
    value: OnceLock<bool>,
    marker: &'static (dyn imp::DynExperimentalOptionMarker + Send + Sync),
}

/// Where an experimental option sits in its life-cycle.
///
/// This shows how stable an experimental option is.
/// Highly unstable options should be marked as `Unstable`.
/// If the API is unlikely to change but still not quite right, use `StableOptIn`.
/// If the option seems correct, mark it as `StableOptOut` so all users get it by default.
/// That's usually the last step before full stabilization.
///
/// If we plan to remove an option, mark it as `Deprecated`.
/// It will trigger a warning when used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stability {
    /// Likely to change, disabled by default.
    Unstable,
    /// Final API, disabled by default.
    StableOptIn,
    /// Final API, enabled by default.
    StableOptOut,
    /// Deprecated, will be removed and prints a warning.
    Deprecated,
}
