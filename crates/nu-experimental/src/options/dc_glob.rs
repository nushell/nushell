use crate::*;

/// Enable `dc-glob` as the glob expansion backend used by command-level glob expansion.
///
/// This keeps the current behavior as default and allows opt-in evaluation of the new backend.
pub static DC_GLOB: ExperimentalOption = ExperimentalOption::new(&DcGlob);

struct DcGlob;

impl ExperimentalOptionMarker for DcGlob {
    const IDENTIFIER: &'static str = "dc-glob";
    const DESCRIPTION: &'static str = "Use dc-glob as the experimental glob expansion backend.";
    const STATUS: Status = Status::OptIn;
    const SINCE: Version = (0, 112, 3);
    const ISSUE: u32 = 18101;
}
