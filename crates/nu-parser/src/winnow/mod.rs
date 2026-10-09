//! The `winnow-parser` experimental front end.
//!
//! With the `winnow-parser` experimental option on, the syntax of every block and module body is
//! parsed by `nu-winnow-parser` instead of this crate's lexer and parser, and the resulting
//! syntax tree is lowered into the `nu-protocol` AST that the rest of Nushell consumes. Nothing
//! downstream changes: blocks are compiled, captures discovered and errors reported exactly as
//! for the classic front end.
//!
//! The pieces:
//!
//! * [`driver`] parses a block with `nu_winnow_parser::BlockStatements`, one statement at a
//!   time, and hands each statement to the lowering (or to the classic parser, see below). A
//!   `use` therefore affects how the following statements resolve their command names, as in
//!   the classic parser. In a long block the statements are parsed on a second thread, ahead of
//!   the lowering, with a copy of the command names; every name it resolved is checked against
//!   the live working set before its statement is lowered.
//! * [`lookup`] answers the winnow parser's questions about which commands exist from the live
//!   [`StateWorkingSet`](nu_protocol::engine::StateWorkingSet), or from a copy of its names for
//!   the second thread.
//! * [`stats`] counts what went where, so a benchmark can say how much of its input the winnow
//!   front end parsed.
//!
//! A statement goes to the classic parser (lexed and parsed from its span, with its effects
//! applied as usual) when the winnow parser reported an error in it, so error messages stay the
//! classic ones. A block the winnow parser cannot lex at all is parsed entirely by the classic
//! parser.

pub(super) mod driver;
mod lookup;
mod lower;
mod stats;

use nu_protocol::{Span, ast::Block, engine::StateWorkingSet};

pub use stats::{WinnowStats, winnow_stats};

/// Whether the `winnow-parser` experimental option is on.
#[inline]
pub(crate) fn enabled() -> bool {
    nu_experimental::WINNOW_PARSER.get()
}

/// The top-level block of a file (the part of `parse` that the classic parser does with `lex`
/// and `parse_block`). `None` when the winnow parser cannot take the file (it is not UTF-8, or
/// it does not lex), in which case nothing was changed and the caller parses it the classic way.
pub(crate) fn parse_file_block(
    working_set: &mut StateWorkingSet,
    contents: &[u8],
    span: Span,
    scoped: bool,
) -> Option<Block> {
    let Ok(source) = std::str::from_utf8(contents) else {
        stats::record_classic_block();
        return None;
    };
    driver::parse_block(
        working_set,
        source,
        span.start,
        span,
        span,
        driver::BlockKind::Block {
            scoped,
            is_subexpression: false,
            input_type: None,
        },
    )
    .map(|(block, _)| block)
}

/// A module's body (`parse_module_block`): its block, the module it defines and the comments at
/// its top. `None` when the winnow parser cannot take it; nothing was changed then.
pub(crate) fn parse_module_block(
    working_set: &mut StateWorkingSet,
    span: Span,
    module_name: &[u8],
) -> Option<(Block, nu_protocol::Module, Vec<Span>)> {
    // The source is copied: the syntax tree borrows it while the working set changes.
    let Ok(source) = String::from_utf8(working_set.get_span_contents(span).to_vec()) else {
        stats::record_classic_block();
        return None;
    };
    let (block, module) = driver::parse_block(
        working_set,
        &source,
        span.start,
        span,
        span,
        driver::BlockKind::Module {
            name: module_name.to_vec(),
        },
    )?;
    let module = module?;
    let comments = first_comments(&source, span.start);
    Some((block, module, comments))
}

/// The comments at the top of a module file, which document the module (the classic parser's
/// `collect_first_comments`, over the source text): consecutive comment lines up to the first
/// blank line after them, none if code comes first. A shebang line is skipped.
fn first_comments(source: &str, offset: usize) -> Vec<Span> {
    let mut comments = Vec::new();
    let mut line_start = 0;
    for line in source.split_inclusive('\n') {
        let start = line_start;
        line_start += line.len();
        let text = line.trim_end_matches('\n');
        let trimmed = text.trim_start_matches([' ', '\t', '\r']);
        if trimmed.is_empty() {
            if comments.is_empty() {
                continue;
            }
            break;
        }
        if !trimmed.starts_with('#') {
            comments.clear();
            break;
        }
        if comments.is_empty() && trimmed.starts_with("#!") {
            continue;
        }
        let comment_start = start + (text.len() - trimmed.len());
        comments.push(Span::new(
            offset + comment_start,
            offset + start + text.len(),
        ));
    }
    comments
}
