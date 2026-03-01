use crate::*;

/// Enable type inferencing on cell paths
pub static CELL_PATH_TYPES: ExperimentalOption = ExperimentalOption::new(&CellPathTypes);

struct CellPathTypes;

impl ExperimentalOptionMarker for CellPathTypes {
    const IDENTIFIER: &'static str = "cell-path-types";
    const DESCRIPTION: &'static str = "\
        Enforce type inferencing of cell paths at parse time. \
        It might result in degraded performance.";
    const STATUS: Status = Status::OptIn;
    const SINCE: Version = (0, 111, 0);
    const ISSUE: u32 = 17683;
}
