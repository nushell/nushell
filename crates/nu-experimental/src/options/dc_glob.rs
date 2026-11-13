use crate::*;

/// Enable `dc-glob` as the glob expansion backend used by command-level glob expansion.
///
/// This keeps the current behavior as default and allows opt-in evaluation of the new backend.
pub static DC_GLOB: ExperimentalOption = ExperimentalOption::new(&DcGlob);

struct DcGlob;

impl ExperimentalOptionMarker for DcGlob {
    const IDENTIFIER: &'static str = "dc-glob";
    const DESCRIPTION: &'static str = "Use dc-glob as the glob expansion backend for every command, `glob` included. It follows nu-glob's rules, so `ls`, `rm`, `cp`, `mv`, `du`, `open` and external arguments return the same paths, and it adds `{a,b}` alternatives. For the `glob` command, which used wax, `dir/**` now lists the directories below `dir`, and wax's `<x:n,m>` repeats and `\\` escapes are gone.";
    const STATUS: Status = Status::OptIn;
    const SINCE: Version = (0, 112, 3);
    const ISSUE: u32 = 18101;
}
