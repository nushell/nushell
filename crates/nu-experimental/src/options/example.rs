use crate::*;

/// Example experimental option.
/// 
/// This is an example how experimental options should be implemented and documented.
/// Just from reading the documentation about this static value should make it clear what 
/// experimental feature is supposed to do and it should be used in the rest of our codebase. 
pub static EXAMPLE: ExperimentalOption = ExperimentalOption::new(&Example);

// We don't need to document this type as it is not public.
// Only the static should be documented, but extensively.
struct Example;

impl ExperimentalOptionMarker for Example {
    const IDENTIFIER: &'static str = "example";
    const DESCRIPTION: &'static str = "This is an example of an experimental option.";
    const STABILITY: Stability = Stability::Unstable;
}
