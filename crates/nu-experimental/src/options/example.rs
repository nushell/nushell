use crate::*;

/// Example experimental option.
///
/// This shows how experimental options should be implemented and documented.
/// Reading this static's documentation alone should clearly explain what the
/// option changes and how it interacts with the rest of the codebase.
///
/// Use this pattern when adding real experimental options.
pub static EXAMPLE: ExperimentalOption = ExperimentalOption::new(&Example);

// No documentation needed here since this type isn't public.
// The static above provides all necessary details.
struct Example;

impl ExperimentalOptionMarker for Example {
    const IDENTIFIER: &'static str = "example";
    const DESCRIPTION: &'static str = "This is an example of an experimental option.";
    const STATUS: Status = Status::DeprecatedDiscard;
}
