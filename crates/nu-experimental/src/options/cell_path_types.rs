use crate::*;

/// Enable type inferencing on "full cell path" expressions which are all typed as `any` if
/// disabled. Inferred types are stored on internal AST, which are helpful for some downstream
/// tasks such as nu-lsp inlay hints.
/// As a side effect, some previously allowed operations may get rejected with type-mismatch parsing errors.
pub static CELL_PATH_TYPES: ExperimentalOption = ExperimentalOption::new(&CellPathTypes);

struct CellPathTypes;

impl ExperimentalOptionMarker for CellPathTypes {
    const IDENTIFIER: &'static str = "cell-path-types";
    const DESCRIPTION: &'static str = "\
        Enforce type inferencing of cell paths at parse time. \
        It might result in degraded performance.";
    const STATUS: Status = Status::OptIn;
    const SINCE: Version = (0, 111, 1);
    const ISSUE: u32 = 17683;
}
