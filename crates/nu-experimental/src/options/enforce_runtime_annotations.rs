use crate::*;

/// Enable pipefail feature to ensure that the exit status of a pipeline
/// accurately reflects the success or failure of all commands within that pipeline, not just
/// the last one.
///
/// So it helps user writing more rubost nushell script.
pub static ENFORCE_RUNTIME_ANNOTATIONS: ExperimentalOption = ExperimentalOption::new(&EnforceRuntimeAnnotations);

// No documentation needed here since this type isn't public.
// The static above provides all necessary details.
struct EnforceRuntimeAnnotations;

impl ExperimentalOptionMarker for EnforceRuntimeAnnotations {
    const IDENTIFIER: &'static str = "enforce-runtime-annotations";
    const DESCRIPTION: &'static str = "\
        Enforce type checking of let assignments at runtime such that \
        invalid type conversion errors propagate the same way they would for predefined values.";
    const STATUS: Status = Status::OptIn;
    const SINCE: Version = (0, 107, 1);
    const ISSUE: u32 = 16832;
}
