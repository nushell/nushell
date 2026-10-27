use crate::*;

/// Parse with nu-parser's winnow front end (`nu_parser::winnow`).
///
/// The syntax of each block and module body is parsed by the winnow parser
/// (`nu_parser::winnow::syntax`), one statement at a time, and a lowering pass in `nu-parser` turns
/// each statement into the same `nu-protocol` AST that `nu-parser` builds: it resolves commands and
/// variables, applies signatures, infers and checks types, and compiles closures. A `use` loads
/// its module with `nu-parser`'s code; the other statements that link modules (`module`,
/// `overlay`, `hide`, `source`) are parsed by `nu-parser` (the module bodies they load come back
/// to the winnow front end), and so is any statement with an error, so errors keep their
/// messages. This exists to measure whether the winnow front end is faster.
pub static WINNOW_PARSER: ExperimentalOption = ExperimentalOption::new(&WinnowParser);

// No documentation needed here since this type isn't public.
// The static above provides all necessary details.
struct WinnowParser;

impl ExperimentalOptionMarker for WinnowParser {
    const IDENTIFIER: &'static str = "winnow-parser";
    const DESCRIPTION: &'static str = "\
        Parse with the winnow parser in nu-parser and lower its syntax tree into nu-parser's AST, \
        to compare the two parsers.";
    const STATUS: Status = Status::OptIn;
    const SINCE: Version = (0, 116, 2);
    const ISSUE: u32 = 0;
}
