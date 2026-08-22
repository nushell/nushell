use crate::*;

/// Preserve structured pipeline data between a parent `nu` and a child `nu`.
///
/// When enabled, `run-external` detects child Nushell processes and exchanges NUON
/// on stdin/stdout instead of pretty-printed tables. The child is notified through
/// the [`STRUCTURED_IO_ENV`] environment variable.
pub static STRUCTURED_IO: ExperimentalOption = ExperimentalOption::new(&StructuredIo);

struct StructuredIo;

impl ExperimentalOptionMarker for StructuredIo {
    const IDENTIFIER: &'static str = "structured-io";
    const DESCRIPTION: &'static str = "\
        When nu is spawned from nu, pass structured pipeline data (NUON) between \
        parent and child instead of pretty-printed tables.";
    const STATUS: Status = Status::OptIn;
    const SINCE: Version = (0, 115, 1);
    const ISSUE: u32 = 3551;
}
