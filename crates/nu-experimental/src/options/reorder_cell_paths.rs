use crate::*;

/// Reorder cell-path members in `Value::follow_cell_path` to decrease memory usage.
///
/// - Accessing a field in a record is cheap, just a simple search and you get a reference to the
///   value.
/// - Accessing an row in a list is even cheaper, just an array access and you get a reference to
///   the value.
/// - Accessing a **column** in a table is expensive, it requires accessing the relevant field in
///   all rows, and creating a new list to store those value. This uses more computation and more
///   memory than the previous two operations.
///
/// Thus, accessing a column first, and then immediately accessing a row of that column is very
/// wasteful. By simply prioritizing row accesses over column accesses whenever possible we can
/// significantly reduce time and memory use.
pub static REORDER_CELL_PATHS: ExperimentalOption = ExperimentalOption::new(&ReorderCellPaths);

// No documentation needed here since this type isn't public.
// The static above provides all necessary details.
struct ReorderCellPaths;

impl ExperimentalOptionMarker for ReorderCellPaths {
    const IDENTIFIER: &'static str = "reorder-cell-paths";
    const DESCRIPTION: &'static str = "\
        Reorder the steps in accessing nested value to decrease memory usage.

\
        Reorder the parts of cell-path when accessing a cell in a table, always select the row before selecting the column.";
    const STATUS: Status = Status::OptIn;
    const SINCE: Version = (0, 105, 2);
    const ISSUE: u32 = 16766;
}
