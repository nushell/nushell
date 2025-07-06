#![allow(
    private_interfaces,
    reason = "The marker structs don't need to be exposed, only the static values."
)]

use crate::*;

mod example;
mod reorder_cell_paths;

/// Marker trait for defining experimental options.
///
/// Implement this trait to mark a struct as metadata for an [`ExperimentalOption`].
/// It provides all necessary information about an experimental feature directly in code,
/// without needing external documentation.
///
/// The `STATUS` field is especially important as it controls whether the feature is enabled
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

    /// Indicates the status of an experimental status.
    ///
    /// Options marked [`Status::OptIn`] are disabled by default while options marked with
    /// [`Status::OptOut`] are enabled by default.
    /// Experimental options that stabilize should be marked as [`Status::DeprecatedDefault`] while
    /// options that will be removed should be [`Status::DeprecatedDiscard`].
    const STATUS: Status;
}

// Export only the static values.
// The marker structs are not relevant and needlessly clutter the generated docs.
pub use example::EXAMPLE;
pub use reorder_cell_paths::REORDER_CELL_PATHS;

// Include all experimental option statics in here.
// This will test them and add them to the parsing list.

/// A list of all available experimental options.
///
/// Use this to show users every experimental option, including their descriptions,
/// identifiers, and current state.
pub static ALL: &[&ExperimentalOption] = &[&EXAMPLE, &REORDER_CELL_PATHS];

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn assert_identifiers_are_unique() {
        let list: Vec<_> = ALL.iter().map(|opt| opt.identifier()).collect();
        let set: HashSet<_> = HashSet::from_iter(&list);
        assert_eq!(list.len(), set.len());
    }

    #[test]
    fn assert_identifiers_are_valid() {
        for option in ALL {
            let identifier = option.identifier();
            assert!(!identifier.is_empty());

            let mut chars = identifier.chars();
            let first = chars.next().expect("not empty");
            assert!(first.is_alphabetic());
            assert!(first.is_lowercase());

            for char in chars {
                assert!(char.is_alphanumeric() || char == '-');
                if char.is_alphabetic() {
                    assert!(char.is_lowercase());
                }
            }
        }
    }

    #[test]
    fn assert_description_not_empty() {
        for option in ALL {
            assert!(!option.description().is_empty());
        }
    }
}
