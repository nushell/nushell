//! Lexing each file's bracket structure once.
//!
//! The parser lexes the inside of every list, record, closure, block and subexpression again when
//! it parses that construct, and every one of those lexes scans the bytes of all the groups nested
//! inside it to find where they close. Without help, a byte would be scanned once per nesting
//! level. Instead, the first lex of a whole file records where each of its groups closes (a
//! [`BracketTable`]), and later lexes of parts of the file jump over nested groups instead of
//! scanning them again (see [`Brackets`](crate::lex::Brackets)).
//!
//! That is exact because, once the lexer is inside a group (not in a string or comment), where the
//! group closes depends only on the bytes from its opening bracket on, never on the lexer options a
//! caller passes (`additional_whitespace`, `special_tokens`, `skip_comment`): those only apply at
//! the top level of a token. So a close recorded by one lex holds for every other lex that reaches
//! the same opening bracket inside a group. The exception is signature lexing, where `<` and `>`
//! nest too, so it neither records nor jumps.

use crate::lex::{LexState, NoBrackets, RecordBrackets, Token, lex_n_tokens_with, lex_with};
use nu_protocol::{
    ParseError, Span,
    engine::{BracketTable, StateWorkingSet},
};
use std::cell::Cell;

/// The sizes of files that get a bracket table; other files are lexed by scanning every group.
/// Below the minimum, recording and looking up a table costs more than the
/// little nesting a small file can have saves (measured on the default config files, 400-500
/// bytes); above the maximum, the table (four bytes per source byte) gets too large.
const BRACKET_TABLE_LEN: std::ops::RangeInclusive<usize> = 1024..=16 * 1024 * 1024;

/// Lex a whole file (or part of one) as [`lex`](crate::lex::lex) does. When
/// [`StateWorkingSet::lex_once`] is on, this records the file's [`BracketTable`] while lexing it,
/// or jumps over nested groups if the table is already there.
///
/// A table serves only the parse of its file: the caller (`parse` or `parse_module_block`)
/// truncates [`StateWorkingSet::bracket_tables`] back to its length before this call when it is
/// done with the file. So the tables are those of the files being parsed, innermost last, and a
/// working set that parses many files one after another (nu-lsp's workspace search) does not keep
/// a table for each of them.
pub(crate) fn lex_file(
    working_set: &mut StateWorkingSet,
    span: Span,
    additional_whitespace: &[u8],
    special_tokens: &[u8],
    skip_comment: bool,
) -> (Vec<Token>, Option<ParseError>) {
    let is_new_file = working_set.lex_once
        && find_bracket_table(working_set, span).is_none()
        && BRACKET_TABLE_LEN.contains(&span.end.saturating_sub(span.start))
        && working_set
            .find_file_by_span(span)
            .is_some_and(|file| file.covered_span == span);
    if !is_new_file {
        return lex_span(
            working_set,
            span,
            additional_whitespace,
            special_tokens,
            skip_comment,
        );
    }

    let (lexed, table) = record_bracket_table(
        working_set.get_span_contents(span),
        span.start,
        additional_whitespace,
        special_tokens,
        skip_comment,
    );
    working_set.bracket_tables.push(table);
    lexed
}

/// Lex `input`, the contents of a file whose covered span starts at `start`, as
/// [`lex`](crate::lex::lex) does, and record where each of its groups closes.
fn record_bracket_table(
    input: &[u8],
    start: usize,
    additional_whitespace: &[u8],
    special_tokens: &[u8],
    skip_comment: bool,
) -> ((Vec<Token>, Option<ParseError>), BracketTable) {
    let mut close = vec![0; input.len()].into_boxed_slice();
    let lexed = lex_with(
        input,
        start,
        additional_whitespace,
        special_tokens,
        skip_comment,
        RecordBrackets {
            start,
            close: Cell::from_mut(&mut close[..]).as_slice_of_cells(),
        },
    );
    let table = BracketTable {
        covered_span: Span::new(start, start + input.len()),
        close,
    };
    (lexed, table)
}

/// Lex part of a file as [`lex`](crate::lex::lex) does, jumping over nested groups with the
/// file's [`BracketTable`] if [`lex_file`] recorded one.
pub(crate) fn lex_span(
    working_set: &StateWorkingSet,
    span: Span,
    additional_whitespace: &[u8],
    special_tokens: &[u8],
    skip_comment: bool,
) -> (Vec<Token>, Option<ParseError>) {
    let input = working_set.get_span_contents(span);
    match find_bracket_table(working_set, span) {
        Some(table) => lex_with(
            input,
            span.start,
            additional_whitespace,
            special_tokens,
            skip_comment,
            table,
        ),
        None => lex_with(
            input,
            span.start,
            additional_whitespace,
            special_tokens,
            skip_comment,
            NoBrackets,
        ),
    }
}

/// [`lex_n_tokens`](crate::lex::lex_n_tokens), jumping over nested groups with `table` if there is
/// one (from [`find_bracket_table`] for the span `state` lexes).
pub(crate) fn lex_n_tokens_in(
    state: &mut LexState,
    additional_whitespace: &[u8],
    special_tokens: &[u8],
    skip_comment: bool,
    max_tokens: usize,
    table: Option<&BracketTable>,
) -> isize {
    match table {
        Some(table) => lex_n_tokens_with(
            state,
            additional_whitespace,
            special_tokens,
            skip_comment,
            max_tokens,
            table,
        ),
        None => lex_n_tokens_with(
            state,
            additional_whitespace,
            special_tokens,
            skip_comment,
            max_tokens,
            NoBrackets,
        ),
    }
}

/// The recorded bracket table of the file containing `span`, if [`StateWorkingSet::lex_once`] is
/// on. The tables are those of the files being parsed, innermost last (see [`lex_file`]), and the
/// innermost file is the one being lexed, so the search starts there.
pub(crate) fn find_bracket_table<'a>(
    working_set: &'a StateWorkingSet,
    span: Span,
) -> Option<&'a BracketTable> {
    if !working_set.lex_once {
        return None;
    }
    working_set
        .bracket_tables
        .iter()
        .rev()
        .find(|table| table.covered_span.contains_span(span))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lex::lex;

    /// Lexer settings: `additional_whitespace`, `special_tokens` and `skip_comment`.
    type Settings = (&'static [u8], &'static [u8], bool);

    /// The settings the parser lexes spans with, each distinct combination once.
    const SETTINGS: &[Settings] = &[
        // blocks, closures, bindings, assignments, modules and the top level
        (b"", b"", false),
        // the shorter reading of a command head
        (b"", b"", true),
        // lists, tables, list patterns and binary literals
        (b"\n\r,", b"", true),
        // records and record patterns
        (b"\n\r,", b":", true),
        // match blocks
        (b" \r\n,|", b"", true),
        // the part of a range before `..`
        (b"", b".?!", true),
        // the first tokens of a `{`, to tell a record from a closure
        (b"\r\n\t", b":", true),
        // cell paths
        (b"\n\r", b".?!", true),
        // subexpressions
        (b"\n\r", b"", true),
    ];

    /// The settings the parser lexes a few tokens at a time with (`lex_n_tokens`): record keys and
    /// values, and the first tokens of a `{`.
    const STEP_SETTINGS: &[Settings] = &[
        (b"\n\r,", b":", true),
        (b"\n\r,", b"", true),
        (b"\r\n\t", b":", true),
    ];

    /// Inputs where a jump could go wrong: signature brackets, closers outside the lexed slice,
    /// error labels that depend on the skipped bytes, comments that end differently at the top
    /// level of a token and inside a group, mismatched and stray closers, raw strings, strings
    /// and interpolation, CRLF line endings and bytes that aren't UTF-8.
    const HAZARDS: &[&[u8]] = &[
        b"def foo [a: bool = (1 < 2)] { $a }",
        b"def foo [a: record<b: list<int]>>] {}",
        b"$\"(1 # )\n)\"",
        b"(1..2)..3",
        b"[1 2]..5",
        b"{ [1 2]\n| foo",
        b"def x [\n a\n {",
        b"[1 2 3",
        b"{\n# a\r[\n]}",
        b"[1 2] # c\r[3",
        b"[a,#[b]]",
        b"[a #b]",
        b"a\x0c# [ x",
        b"[a\xa0#[b] c]",
        b"[a\x85#(b) c]",
        b"{ a] }",
        b"b]",
        b"{a: 1} extra}",
        b"let x = { [ ) ] }",
        b"[r#'a]b'# c]",
        b"bar#'x'#",
        b"[r#'open]",
        b"r#'#",
        b"r##'#'##",
        b"r#x [1]",
        b"{ x: [1 (2 + 3)] , y: {z: \"}\"} }",
        b"$\"a (b [c {d}] e) f\"",
        b"$'(foo 'bar')'",
        b"\"abc\\",
        b"`a[b` [`c]`]",
        b"ls\r\n[1 2]\r\n{a: 1}\r\n(3)\r\n",
        b"[\xff (\xfe)] {\xc3 }",
        b"# [x]\n[y]",
        b"x\x0c#[a] (b)",
        b"[1 2] # [3]\n[4]",
    ];

    /// `source` itself and every prefix of it, which is what the REPL lexes while a line is typed.
    fn prefixes(source: &[u8]) -> Vec<Vec<u8>> {
        (0..=source.len()).map(|n| source[..n].to_vec()).collect()
    }

    /// The [`prefixes`] of `source`, and for every bracket, quote and `#`: `source` without it,
    /// and with a stray closer, a comment start, a carriage return, an interpolation start or a
    /// raw string start inserted before it.
    fn mutations(source: &[u8]) -> Vec<Vec<u8>> {
        let mut variants = prefixes(source);
        for (i, byte) in source.iter().enumerate() {
            if !b"()[]{}'\"`#".contains(byte) {
                continue;
            }
            variants.push([&source[..i], &source[i + 1..]].concat());
            for insert in [&b")"[..], b"]", b"}", b" #", b"\r", b"$\"", b"r#'"] {
                variants.push([&source[..i], insert, &source[i..]].concat());
            }
        }
        variants
    }

    /// The bracket table [`lex_file`] records for `source` placed at `offset`. Recording it lexes
    /// `source` exactly as a plain lex does.
    fn record_table(source: &[u8], offset: usize) -> BracketTable {
        let (lexed, table) = record_bracket_table(source, offset, b"", b"", false);
        assert_eq!(
            lexed,
            lex(source, offset, b"", b"", false),
            "recording the table of {:?}",
            String::from_utf8_lossy(source),
        );
        table
    }

    /// Lex `input` with `lex_n_tokens`, `max_tokens` at a time, as records and the brace peek do.
    fn lex_in_steps<B: crate::lex::Brackets>(
        input: &[u8],
        span_offset: usize,
        (additional_whitespace, special_tokens, skip_comment): Settings,
        max_tokens: usize,
        brackets: B,
    ) -> (Vec<Token>, Option<ParseError>) {
        let mut state = LexState {
            input,
            output: vec![],
            error: None,
            span_offset,
        };
        while !state.input.is_empty()
            && lex_n_tokens_with(
                &mut state,
                additional_whitespace,
                special_tokens,
                skip_comment,
                max_tokens,
                brackets,
            ) > 0
        {}
        (state.output, state.error)
    }

    /// Lexing with `source`'s bracket table gives what lexing without it gives: for the whole
    /// source, every group with and without its brackets, and prefixes that cut a group short,
    /// under every setting, all at once and a few tokens at a time.
    fn assert_jumping_lexes_like_scanning(source: &[u8]) {
        // Give the file a place away from 0, like every file after the first.
        let offset = 1000;
        let table = record_table(source, offset);
        let mut slices = vec![(0, source.len())];
        for (open, &close) in table.close.iter().enumerate() {
            if close != 0 {
                let close = close as usize - 1;
                slices.extend([(open, close + 1), (open + 1, close), (open, close)]);
            }
        }
        for (start, end) in slices {
            let input = &source[start..end];
            for &settings in SETTINGS {
                let (additional_whitespace, special_tokens, skip_comment) = settings;
                assert_eq!(
                    lex_with(
                        input,
                        offset + start,
                        additional_whitespace,
                        special_tokens,
                        skip_comment,
                        &table
                    ),
                    lex(
                        input,
                        offset + start,
                        additional_whitespace,
                        special_tokens,
                        skip_comment
                    ),
                    "lexing {:?} with {settings:?}",
                    String::from_utf8_lossy(input),
                );
            }
            for &settings in STEP_SETTINGS {
                for max_tokens in [1, 2] {
                    assert_eq!(
                        lex_in_steps(input, offset + start, settings, max_tokens, &table),
                        lex_in_steps(input, offset + start, settings, max_tokens, NoBrackets),
                        "lexing {:?} with {settings:?}, {max_tokens} token(s) at a time",
                        String::from_utf8_lossy(input),
                    );
                }
            }
        }
    }

    #[test]
    fn jumping_lexes_like_scanning() {
        for hazard in HAZARDS {
            for variant in mutations(hazard) {
                assert_jumping_lexes_like_scanning(&variant);
            }
        }
    }

    /// [`lex_file`] records the table of a whole file of 1 KiB or more, and only of such a file,
    /// and lexes of parts of the file find it.
    #[test]
    fn lex_file_records_the_table_of_a_whole_file() {
        let engine_state = nu_protocol::engine::EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);
        let record = "{ x: (1 + 2) }";
        let source = format!("[{}] {record}", "1 ".repeat(600));
        let file = working_set.add_file("big.nu", source.as_bytes());
        let span = working_set.get_span_for_file(file);

        // Part of the file gets no table.
        lex_file(
            &mut working_set,
            Span::new(span.start, span.start + 1100),
            b"",
            b"",
            false,
        );
        assert!(working_set.bracket_tables.is_empty());

        let (_, error) = lex_file(&mut working_set, span, b"", b"", false);
        assert_eq!(error, None);
        let [table] = working_set.bracket_tables.as_slice() else {
            panic!("one table: {:?}", working_set.bracket_tables);
        };
        assert_eq!(table.covered_span, span);
        assert_eq!(table.close_of(span.start), Some(span.start + 1201));
        let record_span = Span::new(span.end - record.len(), span.end);
        assert!(find_bracket_table(&working_set, record_span).is_some());

        // A whole file under 1 KiB gets none.
        let small = working_set.add_file("small.nu", b"[1 2] { x: 1 }");
        let small_span = working_set.get_span_for_file(small);
        lex_file(&mut working_set, small_span, b"", b"", false);
        assert_eq!(working_set.bracket_tables.len(), 1);
    }
}
