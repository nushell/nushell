//! The `winnow-parser` experimental front end, and the winnow parser itself.
//!
//! With the `winnow-parser` experimental option on, the syntax of every block and module body is
//! parsed by the winnow parser ([`syntax`]) instead of this crate's lexer and parser, and the
//! resulting syntax tree is lowered into the `nu-protocol` AST that the rest of Nushell consumes.
//! Nothing downstream changes: blocks are compiled, captures discovered and errors reported exactly
//! as for the classic front end.
//!
//! The pieces:
//!
//! * [`syntax`] is the winnow parser: a lexer and a parser that build a syntax tree keeping
//!   every byte of the source, written with the `winnow` crate. It asks a
//!   [`CommandLookup`](syntax::CommandLookup) which commands exist. [`docs`] describes it,
//!   chapter by chapter.
//! * `driver` parses a block with `syntax::BlockStatements`, one statement at a time, and
//!   hands each statement to the lowering. A `use` therefore affects how the following
//!   statements resolve their command names, as in the classic parser. In a long block the statements are parsed on a second thread, ahead of
//!   the lowering, with a copy of the command names; every name it resolved is checked against
//!   the live working set before its statement is lowered.
//! * `lookup` answers the winnow parser's questions about which commands exist from the live
//!   [`StateWorkingSet`], or from a copy of its names for the second thread.
//! * `lower` turns each statement's syntax tree into the AST, each of its functions the
//!   counterpart of a classic parser function, or gives the statement back.
//! * `stats` counts what went where, so a benchmark can say how much of its input the winnow
//!   front end parsed.
//!
//! Nothing goes to the classic parser. A statement the winnow front end cannot read (a syntax
//! error the winnow parser did not go past, a construct the lowering does not handle) is a
//! garbage pipeline with an error, and so is a block it cannot read as a whole (text that is
//! not UTF-8, or an error about the whole block).
//!
//! This module is public so that nu-parser's tests, examples and benchmarks and the comparison
//! harness in `winnow/tools` can reach the winnow parser. It is experimental, like the option,
//! and not a stable API.

pub mod docs;
pub(super) mod driver;
mod lookup;
mod lower;
mod stats;
pub mod syntax;

use nu_protocol::{Span, ast::Block, engine::StateWorkingSet};

pub(crate) use stats::record_classic_pipeline;
pub use stats::{WinnowStats, winnow_stats};

/// Runs `parse` and returns, with its result, the winnow syntax tree of each statement of the
/// outermost block the front end read on this thread meanwhile, printed with its diagnostics:
/// what the lowering got. Blocks it read inside those statements (a module a `use` loads) are
/// left out. For tests that pin the tree the shell sees.
pub fn capture_statement_trees<R>(parse: impl FnOnce() -> R) -> (R, Vec<String>) {
    driver::CAPTURE.with_borrow_mut(|capture| *capture = Some(driver::Capture::default()));
    let result = parse();
    let trees = driver::CAPTURE
        .with_borrow_mut(Option::take)
        .map(|capture| capture.trees)
        .unwrap_or_default();
    (result, trees)
}

/// Whether the `winnow-parser` experimental option is on.
#[inline]
pub(crate) fn enabled() -> bool {
    nu_experimental::WINNOW_PARSER.get()
}

/// nu-parser's `parse_block` with the option on: the block that nu-parser's caller lexed as
/// `tokens` (the inside of `( ... )`, a closure's body, a binding's value), parsed and lowered
/// by the winnow front end. `None` with the option off.
pub(crate) fn nested_block(
    working_set: &mut StateWorkingSet,
    tokens: &[crate::Token],
    span: Span,
    scoped: bool,
    is_subexpression: bool,
    input_type: Option<&nu_protocol::Type>,
) -> Option<Block> {
    enabled().then(|| {
        lower::nested_block(
            working_set,
            tokens,
            span,
            scoped,
            is_subexpression,
            input_type,
        )
    })
}

/// The top-level block of a file (the part of `parse` that the classic parser does with `lex`
/// and `parse_block`). Text that is not UTF-8 is an error over a garbage block.
pub(crate) fn parse_file_block(
    working_set: &mut StateWorkingSet,
    contents: &[u8],
    span: Span,
    scoped: bool,
) -> Block {
    // `contents` is the caller's, not the working set's, so the syntax tree can borrow it while
    // the working set changes (unlike a module body's, which `parse_module_block` copies).
    let source = match std::str::from_utf8(contents) {
        Ok(source) => source,
        Err(error) => return non_utf8_block(working_set, span, span.start, error),
    };
    let code = code_span(working_set, source, span);
    driver::parse_block(
        working_set,
        source,
        span.start,
        span,
        code,
        driver::BlockKind::Block {
            scoped,
            of: lower::BlockOf::Code,
            input_type: None,
            caller_lexed: false,
        },
    )
    .0
}

/// The completer's next-shorter reading of the command head at `head`
/// (`parse_shorter_head_reading`): `None` when it has none, or when the winnow front end cannot
/// read the call it would be.
pub(crate) fn parse_shorter_head_reading(
    working_set: &mut StateWorkingSet,
    head: Span,
    input_type: Option<&nu_protocol::Type>,
) -> Option<nu_protocol::ast::Expression> {
    lower::shorter_head_reading(working_set, head, input_type)
}

/// A module's body (`parse_module_block`): its block, the module it defines and the comments at
/// its top. Text that is not UTF-8 is an error over a garbage block, which defines nothing.
pub(crate) fn parse_module_block(
    working_set: &mut StateWorkingSet,
    span: Span,
    module_name: &[u8],
) -> (Block, nu_protocol::Module, Vec<Span>) {
    let empty_module = || nu_protocol::Module::from_span(module_name.to_vec(), span);
    // The source is copied: the syntax tree borrows it while the working set changes.
    let source = match String::from_utf8(working_set.get_span_contents(span).to_vec()) {
        Ok(source) => source,
        Err(error) => {
            let block = non_utf8_block(working_set, span, span.start, error.utf8_error());
            return (block, empty_module(), Vec::new());
        }
    };
    let code = code_span(working_set, &source, span);
    let (block, module) = driver::parse_block(
        working_set,
        &source,
        span.start,
        span,
        code,
        driver::BlockKind::Module {
            name: module_name.to_vec(),
        },
    );
    let module = module.unwrap_or_else(empty_module);
    let comments = first_comments(&source[code.start - span.start..], code.start);
    (block, module, comments)
}

/// The part of `span`, whose text is `source`, that holds code: all of it, but for the UTF-8
/// byte-order mark a whole file may start with (nu-parser's `lex_file`). The file's contents keep
/// the mark, so spans into the file don't move.
fn code_span(working_set: &StateWorkingSet, source: &str, span: Span) -> Span {
    const UTF8_BOM: &str = "\u{feff}";
    if source.starts_with(UTF8_BOM)
        && working_set
            .find_file_by_span(span)
            .is_some_and(|file| file.covered_span == span)
    {
        Span::new(span.start + UTF8_BOM.len(), span.end)
    } else {
        span
    }
}

/// The block covering `span` whose text, from `text_start` on, is not UTF-8 (`error`): the
/// winnow parser reads only UTF-8, so it is an error (NonUtf8) at the first byte that is not,
/// over a garbage block.
fn non_utf8_block(
    working_set: &mut StateWorkingSet,
    span: Span,
    text_start: usize,
    error: std::str::Utf8Error,
) -> Block {
    let start = text_start + error.valid_up_to();
    let end = (start + error.error_len().unwrap_or(1)).min(span.end.max(start));
    working_set.error(nu_protocol::ParseError::NonUtf8(Span::new(start, end)));
    driver::unread_block(working_set, span, span, "not UTF-8")
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
