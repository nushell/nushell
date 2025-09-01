use crate::*;

/// Enable pipefail feature to ensure that the exit status of a pipeline
/// accurately reflects the success or failure of all commands within that pipeline, not just
/// the last one.
///
/// So it helps user writing more rubost nushell script.
pub static PIPE_FAIL: ExperimentalOption = ExperimentalOption::new(&PipeFail);

// No documentation needed here since this type isn't public.
// The static above provides all necessary details.
struct PipeFail;

impl ExperimentalOptionMarker for PipeFail {
    const IDENTIFIER: &'static str = "pipefail";
    const DESCRIPTION: &'static str = "\
        If an external command fails within a pipeline, $env.LAST_EXIT_CODE is set
        to the exit code of rightmost command which failed.";
    const STATUS: Status = Status::OptIn;
}
