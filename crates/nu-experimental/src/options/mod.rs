#![allow(
    private_interfaces,
    reason = "The marker structs don't need to be exposed, only the static values."
)]

use crate::*;

mod example;

/// Marker trait for defining experimental options.
///
/// Implement this trait to mark a struct as metadata for an [`ExperimentalOption`].
/// It provides all necessary information about an experimental feature directly in code,
/// without needing external documentation.
///
/// The `STABILITY` field is especially important as it controls whether the feature is enabled
/// by default and how users should interpret its reliability.
pub(crate) trait ExperimentalOptionMarker {
    /// Unique identifier for this experimental option.
    ///
    /// Must be a valid Rust identifier.
    /// Used when parsing to toggle specific experimental options,
    /// and may also serve as a user-facing label.
    const IDENTIFIER: &'static str;

    /// Brief description explaining what this option changes.
    ///
    /// Displayed to users in help messages or summaries without needing to visit external docs.
    const DESCRIPTION: &'static str;

    /// Indicates how stable or experimental this option is.
    ///
    /// Options marked [`Stability::StableOptOut`] are on by default.
    /// User-facing commands may use this stability information to clarify risk,
    /// particularly highlighting [`Stability::Unstable`] options.
    const STABILITY: Stability;
}

// Export only the static values.
// The marker structs are not relevant and needlessly clutter the generated docs.
pub use example::EXAMPLE;

// Include all experimental option statics in here.
// This will test them and add them to the parsing list.

/// A list of all available experimental options.
///
/// Use this to show users every experimental option, including their descriptions,
/// identifiers, and current state.
pub static ALL: &'static [&ExperimentalOption] = &[&EXAMPLE];
