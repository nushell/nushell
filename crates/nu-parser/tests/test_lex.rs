#![allow(clippy::byte_char_slices)]

use nu_parser::{LexState, Token, TokenContents, lex, lex_n_tokens, lex_signature};
use nu_protocol::{ParseError, Span};
use rstest::rstest;
use std::fmt::Write;

#[test]
fn lex_basic() {
    let file = b"let x = 4";

    let output = lex(file, 0, &[], &[], true);

    assert!(output.1.is_none());
}

#[test]
fn lex_newline() {
    let file = b"let x = 300\nlet y = 500;";

    let output = lex(file, 0, &[], &[], true);

    assert!(output.0.contains(&Token {
        contents: TokenContents::Eol,
        span: Span::new(11, 12)
    }));
}

#[test]
fn lex_annotations_list() {
    let file = b"items: list<string>";

    let (output, err) = lex_signature(file, 0, &[b'\n', b'\r'], &[b':', b'=', b','], false);

    assert!(err.is_none());
    assert_eq!(output.len(), 3);
}

#[test]
fn lex_annotations_record() {
    let file = b"config: record<name: string>";

    let (output, err) = lex_signature(file, 0, &[b'\n', b'\r'], &[b':', b'=', b','], false);

    assert!(err.is_none());
    assert_eq!(output.len(), 3);
}

#[test]
fn lex_annotations_empty() {
    let file = b"items: list<>";

    let (output, err) = lex_signature(file, 0, &[b'\n', b'\r'], &[b':', b'=', b','], false);

    assert!(err.is_none());
    assert_eq!(output.len(), 3);
}

#[test]
fn lex_annotations_space_before_annotations() {
    let file = b"items: list <string>";

    let (output, err) = lex_signature(file, 0, &[b'\n', b'\r'], &[b':', b'=', b','], false);

    assert!(err.is_none());
    assert_eq!(output.len(), 4);
}

#[test]
fn lex_annotations_space_within_annotations() {
    let file = b"items: list< string>";

    let (output, err) = lex_signature(file, 0, &[b'\n', b'\r'], &[b':', b'=', b','], false);

    assert!(err.is_none());
    assert_eq!(output.len(), 3);

    let file = b"items: list<string >";

    let (output, err) = lex_signature(file, 0, &[b'\n', b'\r'], &[b':', b'=', b','], false);

    assert!(err.is_none());
    assert_eq!(output.len(), 3);

    let file = b"items: list< string >";

    let (output, err) = lex_signature(file, 0, &[b'\n', b'\r'], &[b':', b'=', b','], false);

    assert!(err.is_none());
    assert_eq!(output.len(), 3);
}

#[test]
fn lex_annotations_nested() {
    let file = b"items: list<record<name: string>>";

    let (output, err) = lex_signature(file, 0, &[b'\n', b'\r'], &[b':', b'=', b','], false);

    assert!(err.is_none());
    assert_eq!(output.len(), 3);
}

#[test]
fn lex_annotations_nested_unterminated() {
    let file = b"items: list<record<name: string>";

    let (output, err) = lex_signature(file, 0, &[b'\n', b'\r'], &[b':', b'=', b','], false);

    assert!(matches!(
        err.unwrap(),
        ParseError::Unclosed(delim, ..) if delim == ">"
    ));
    assert_eq!(output.len(), 3);
}

#[test]
fn lex_annotations_unterminated() {
    let file = b"items: list<string";

    let (output, err) = lex_signature(file, 0, &[b'\n', b'\r'], &[b':', b'=', b','], false);

    assert!(matches!(
        err.unwrap(),
        ParseError::Unclosed(delim, ..) if delim == ">"
    ));
    assert_eq!(output.len(), 3);
}

#[test]
fn lex_empty() {
    let file = b"";

    let output = lex(file, 0, &[], &[], true);

    assert!(output.0.is_empty());
    assert!(output.1.is_none());
}

#[test]
fn lex_parenthesis() {
    // The whole parenthesis is an item for the lexer
    let file = b"let x = (300 + (322 * 444));";

    let output = lex(file, 0, &[], &[], true);

    assert_eq!(
        output.0.get(3).unwrap(),
        &Token {
            contents: TokenContents::Item,
            span: Span::new(8, 27)
        }
    );
}

#[test]
fn lex_comment() {
    let file = b"let x = 300 # a comment \n $x + 444";

    let output = lex(file, 0, &[], &[], false);

    assert_eq!(
        output.0.get(4).unwrap(),
        &Token {
            contents: TokenContents::Comment,
            span: Span::new(12, 24)
        }
    );
}

#[test]
fn lex_not_comment_needs_space_in_front_of_hashtag() {
    let file = b"1..10 | each {echo test#testing }";

    let output = lex(file, 0, &[], &[], false);

    assert!(output.1.is_none());
}

#[test]
fn lex_comment_with_space_in_front_of_hashtag() {
    let file = b"1..10 | each {echo test #testing }";

    let output = lex(file, 0, &[], &[], false);

    assert!(output.1.is_some());
    // Primary span is the opening `{` of the each block (innermost unclosed).
    assert!(matches!(
        output.1.unwrap(),
        ParseError::Unclosed(missing_token, open, ..) if missing_token == "}"
            && open == Span::new(13, 14)
    ));
}

#[test]
fn lex_comment_with_tab_in_front_of_hashtag() {
    let file = b"1..10 | each {echo test\t#testing }";

    let output = lex(file, 0, &[], &[], false);

    assert!(output.1.is_some());
    assert!(matches!(
        output.1.unwrap(),
        ParseError::Unclosed(missing_token, open, ..) if missing_token == "}"
            && open == Span::new(13, 14)
    ));
}

#[test]
fn lex_is_incomplete() {
    let file = b"let x = 300 | ;";

    let output = lex(file, 0, &[], &[], true);

    let err = output.1.unwrap();
    assert!(matches!(err, ParseError::ExtraTokens(_)));
}

#[test]
fn lex_incomplete_paren() {
    let file = b"let x = (300 + ( 4 + 1)";

    let output = lex(file, 0, &[], &[], true);

    let err = output.1.unwrap();
    // Inner `( 4 + 1)` is closed by the trailing `)`; remaining open is the outer `(`.
    assert!(matches!(
        err,
        ParseError::Unclosed(v, open, ..) if v == ")" && open == Span::new(8, 9)
    ));
}

#[test]
fn lex_incomplete_quote() {
    let file = b"let x = '300 + 4 + 1";

    let output = lex(file, 0, &[], &[], true);

    let err = output.1.unwrap();
    assert!(matches!(
        err,
        ParseError::Unclosed(v, open, ..) if v == "'" && open == Span::new(8, 9)
    ));
}

// ---------------------------------------------------------------------------
// Delimiter diagnostics regression suite
//
// Presentation heuristics may reshape *labels* when the delimiter stack proves
// a real failure.
// ---------------------------------------------------------------------------

fn parse_first_error(src: &[u8]) -> ParseError {
    let engine = nu_protocol::engine::EngineState::new();
    let mut ws = nu_protocol::engine::StateWorkingSet::new(&engine);
    nu_parser::parse(&mut ws, None, src, false);
    assert!(
        !ws.parse_errors.is_empty(),
        "expected a parse error for {:?}",
        std::str::from_utf8(src)
    );
    ws.parse_errors[0].clone()
}

fn line_of(src: &[u8], offset: usize) -> usize {
    src[..offset.min(src.len())]
        .iter()
        .filter(|&&b| b == b'\n')
        .count()
        + 1
}

fn snippet_at(src: &[u8], span: Span) -> &str {
    let end = span.end.min(src.len()).max(span.start);
    std::str::from_utf8(&src[span.start..end]).unwrap_or("")
}

/// Assert reshape diagnostic: primary = closer, secondary = insert-site hint.
/// Returns `(closer_span, hint_span)`.
fn assert_unexpected_closer(err: &ParseError, closer: &str, hint_label_sub: &str) -> (Span, Span) {
    match err {
        ParseError::UnexpectedCloser {
            closer: c,
            closer_span,
            hint_span,
            hint_label,
            help,
        } => {
            assert_eq!(*c, closer, "unexpected closer kind in {err:?}");
            let hint = hint_span.expect("expected secondary insert-site hint");
            assert!(
                hint_label.contains(hint_label_sub),
                "hint label missing {hint_label_sub:?}: {hint_label}"
            );
            // Reshape diagnostics must offer dual repair (remove closer or add opener).
            assert!(
                help.contains("if it is extra") && help.contains("or add"),
                "help should be dual remove/add, got: {help}"
            );
            (*closer_span, hint)
        }
        other => panic!(
            "expected UnexpectedCloser({closer:?}) with hint containing {hint_label_sub:?}, got {other:?}"
        ),
    }
}

fn assert_unbalanced(err: &ParseError, open: &str, close: &str) {
    match err {
        ParseError::Unbalanced(o, c, ..) => {
            assert_eq!(*o, open, "unexpected open kind in {err:?}");
            assert_eq!(*c, close, "unexpected close kind in {err:?}");
        }
        other => panic!("expected Unbalanced({open}, {close}), got {other:?}"),
    }
}

fn assert_unclosed(err: &ParseError, delim: &str) -> (Span, Span) {
    match err {
        ParseError::Unclosed(d, open, expected, ..) => {
            assert_eq!(*d, delim, "unexpected unclosed delim in {err:?}");
            (*open, *expected)
        }
        other => panic!("expected Unclosed({delim}), got {other:?}"),
    }
}

#[test]
fn lex_unclosed_nested_brace_points_at_inner_open() {
    // Missing close for `ls: {` with no trailing closers — primary is that opener.
    let file = b"$env.config = {\n  ls: {\n    use_ls_colors: true\n";

    let output = lex(file, 0, &[], &[], true);
    let err = output.1.expect("expected unclosed delimiter error");
    let (open, _end) = assert_unclosed(&err, "}");
    assert_eq!(file[open.start], b'{');
    // Prefer the inner open: second `{` (the `ls` record).
    let second = file
        .iter()
        .enumerate()
        .filter(|(_, b)| **b == b'{')
        .nth(1)
        .map(|(i, _)| i)
        .unwrap();
    assert_eq!(open.start, second);
    if let ParseError::Unclosed(.., help) = &err {
        assert!(
            help.contains("ls") || help.contains("matching"),
            "help should mention structure or generic fix: {help}"
        );
    }
}

// Structure-hint unit tests live next to `delimiter_structure_hint` (crate-private).

/// These patterns previously caused *false positive* Unclosed errors when
/// indent heuristics invented failures. Stack-only rules must accept them.
#[rstest]
#[case::paren_wrapped_record_dedented_closer(
    b"def f [] {\n        let emoji_dict = ({\n        \"200\": \"x\",\n    })\n}\n"
)]
#[case::else_if_chain_dedented_closers(
    b"{||\n    if $in < 1hr {\n      'red'\n      } else if $in < 1wk {\n      'green'\n    } else if $in < 6wk {\n      'blue'\n    } else { 'gray' }\n  }\n"
)]
#[case::balanced_nested_records(
    b"$env.config = {\n  hooks: {\n    pre: 1\n  }\n  rm: {\n    x: 1\n  }\n}\n"
)]
#[case::balanced_list_paren_record_mix(b"let x = [1 (2 + 3) {a: 4}]\n")]
#[case::balanced_def_signature_and_parens(
    b"def f [a: int, b: string] { $a + ($b | str length) }\n"
)]
#[case::multiline_pipeline_no_invented_missing_brace(b"ls\n| where type == file\n| get name\n")]
#[case::continued_pipeline_after_pipe(b"ls |\n  get name\n")]
#[case::balanced_quoted_record_keys(b"{ \"type\": 1, name: 2 }\n")]
// Multi-line constructs with matching closers are valid in scripts and the REPL.
#[case::multiline_closed_double_quoted_string(b"let x = \"hello\nworld\"\n")]
#[case::multiline_closed_single_quoted_string(b"let x = 'hello\nworld'\n")]
#[case::multiline_closed_list(b"let y = [1, 2, 3\n4, 5, 6]\n")]
#[case::multiline_closed_list_with_nested(b"let y = [\n  1,\n  (2 + 3),\n  {a: 4}\n]\n")]
#[case::multiline_closed_record(b"let r = {\n  a: 1\n  b: 2\n}\n")]
#[case::multiline_closed_parens(b"let n = (\n  1 + 2\n)\n")]
fn lex_valid_code_never_errors_from_indent_style(#[case] file: &[u8]) {
    let output = lex(file, 0, &[], &[], true);
    assert!(
        output.1.is_none(),
        "valid input must not lex-error, got {:?} for {:?}",
        output.1,
        std::str::from_utf8(file)
    );
    let engine = nu_protocol::engine::EngineState::new();
    let mut ws = nu_protocol::engine::StateWorkingSet::new(&engine);
    nu_parser::parse(&mut ws, None, file, false);
    let delimiterish = ws.parse_errors.iter().any(|e| match e {
        ParseError::Unclosed(..)
        | ParseError::Unbalanced(..)
        | ParseError::UnexpectedCloser { .. } => true,
        ParseError::LabeledErrorWithHelp { error, .. } => error.contains("Missing `"),
        _ => false,
    });
    assert!(
        !delimiterish,
        "valid input must not get delimiter diagnostics, got {:?} for {:?}",
        ws.parse_errors,
        std::str::from_utf8(file)
    );
}

#[test]
fn lex_truly_unclosed_still_reports() {
    // Real stack failure: missing closers at end of input.
    let file = b"$env.config = {\n  ls: {\n    use_ls_colors: true\n";
    let output = lex(file, 0, &[], &[], true);
    let err = output.1.expect("expected real unclosed error");
    assert_unclosed(&err, "}");
}

#[rstest]
#[case::unclosed_paren(b"print (1 + 2", b'(', ")")]
#[case::unclosed_bracket(b"let x = [1, 2", b'[', "]")]
// Multi-line without a closer is still unclosed (not confused with valid multi-line forms).
#[case::multiline_unclosed_list(b"let y = [1, 2, 3\n4, 5, 6", b'[', "]")]
#[case::multiline_unclosed_paren(b"let n = (\n  1 + 2", b'(', ")")]
#[case::multiline_unclosed_record(b"let r = {\n  a: 1\n  b: 2", b'{', "}")]
fn lex_unclosed_paren_and_bracket_report_correct_delim(
    #[case] src: &[u8],
    #[case] open_byte: u8,
    #[case] expected_closer: &str,
) {
    let (open, _) = assert_unclosed(
        &lex(src, 0, &[], &[], true)
            .1
            .expect("expected unclosed delimiter"),
        expected_closer,
    );
    assert_eq!(src[open.start], open_byte);
}

#[test]
fn lex_missing_closure_brace_before_pipe_labels_near_pipe() {
    // Real bug pattern from defs.nu `startup-stats`: forgot `}` after a closure
    // body before the next `| upsert`. Stack still fails (outer `{` unclosed);
    // labels should point near the missing closer, not only at EOF.
    let file =
        b"def f [] {\n  ls | upsert a {|n|\n    $n | length\n  | upsert b {|x|\n    $x\n  }\n";
    // Missing `}` after `length` before `| upsert b`. Final braces incomplete.
    let output = lex(file, 0, &[], &[], true);
    let err = output.1.expect("expected real unclosed error");
    let (open, expected) = assert_unclosed(&err, "}");
    // Expected closer should be at the `|` that starts `| upsert b`, not far past it.
    let pipe_at = file
        .windows(10)
        .position(|w| w == b"| upsert b")
        .expect("| upsert b");
    assert_eq!(
        expected.start, pipe_at,
        "expected closer label at `| upsert b` (offset {pipe_at}), open={open:?} expected={expected:?}"
    );
}

#[test]
fn parse_missing_if_open_brace_points_at_condition() {
    // Real stack failure is an extra `}`; secondary label toward the missing `{`
    // after `if` (defs.nu `env-nu` pattern). Primary stays on the closer.
    let file = b"def f [] {\n  each {|r|\n    if $x != string\n      $r\n    }\n  }\n}\n";
    let err = parse_first_error(file);
    let (closer, hint) = assert_unexpected_closer(&err, "}", "possible place for `{`");
    assert_eq!(
        file[closer.start], b'}',
        "primary must be the unmatched closer"
    );
    assert_eq!(
        line_of(file, hint.start),
        3,
        "secondary should land on if-condition line"
    );
    assert!(
        snippet_at(file, hint).contains('g') || file[hint.start] == b'g',
        "should point near end of `string`, got {:?}",
        snippet_at(file, hint)
    );
}

/// while / try / for / match / else-if should get the same unexpected-`}` presentation.
#[rstest]
#[case::while_missing_brace(b"def f [] {\n  while $true\n    1\n  }\n}\n")]
#[case::try_missing_brace(b"def f [] {\n  try\n    1\n  }\n}\n")]
#[case::for_missing_brace(b"def f [] {\n  for x in 1..2\n    $x\n  }\n}\n")]
#[case::match_missing_brace(b"def f [] {\n  match $x\n    1 => { 2 }\n  }\n}\n")]
// `else if` must begin the line (a leading `}` on the same line is a different
// token shape and is not reshaped by this lookback).
#[case::else_if_missing_brace(b"def f [] {\n  if $true { 1 }\n  else if $false\n    2\n  }\n}\n")]
fn parse_missing_open_brace_control_flow_variants(#[case] file: &[u8]) {
    let err = parse_first_error(file);
    let (closer, hint) = assert_unexpected_closer(&err, "}", "possible place for `{`");
    assert_eq!(
        file[closer.start], b'}',
        "primary must be the unmatched closer"
    );
    assert!(
        line_of(file, hint.start) <= 5,
        "hint too far from condition, line {} err={err:?}",
        line_of(file, hint.start)
    );
}

#[test]
fn parse_missing_record_open_brace_points_at_field() {
    // `type: $lst.0}` forgot `{` before the field; the early `}` closes the
    // each-block and a later `}` looks unbalanced far below (defs.nu view std).
    let file = b"\
def f [] {\n\
    insert content {\n\
        each {|lst|\n\
            type: $lst.0}\n\
            | if $true {\n\
                merge {name: x}\n\
            } else {\n\
                merge {name: y}\n\
            }\n\
        }\n\
        | flatten\n\
    }\n\
}\n";
    let err = parse_first_error(file);
    let (closer, hint) = assert_unexpected_closer(&err, "}", "possible place for `{`");
    assert_eq!(
        file[closer.start], b'}',
        "primary must be the unmatched closer"
    );
    assert_eq!(
        line_of(file, hint.start),
        4,
        "secondary should land on bare-record line"
    );
    assert_eq!(
        snippet_at(file, hint).chars().next(),
        Some('t'),
        "should point at `type`, got {:?}",
        snippet_at(file, hint)
    );
}

#[test]
fn parse_extra_brace_without_hint_stays_unbalanced() {
    // Unexpected `}` with no nearby control-flow/bare-record pattern.
    let file = b"def f [] {\n  1\n}\n}\n";
    let err = parse_first_error(file);
    assert_unbalanced(&err, "{", "}");
}

/// Lookback must ignore control-flow-looking text inside multiline strings.
/// Review: https://github.com/nushell/nushell/pull/18702#issuecomment-5075660966
#[test]
fn parse_if_inside_multiline_string_does_not_reshape() {
    let file = b"\
def f [] {\n\
  let s = 'first\n\
if this is just string content\n\
last'\n\
}\n\
}\n";
    let err = parse_first_error(file);
    match &err {
        ParseError::Unbalanced(_, close, close_span, _) => {
            assert_eq!(*close, "}");
            assert_eq!(file[close_span.start], b'}');
            // Final extra `}` is on the last non-empty line.
            assert_eq!(line_of(file, close_span.start), 6);
        }
        ParseError::UnexpectedCloser {
            closer,
            closer_span,
            hint_span,
            hint_label,
            ..
        } => {
            assert_eq!(*closer, "}");
            assert_eq!(file[closer_span.start], b'}');
            // If a hint exists, it must not point at the string's `if` line.
            if let Some(hint) = hint_span {
                assert_ne!(
                    line_of(file, hint.start),
                    3,
                    "must not label string content as insert site: {hint_label}"
                );
            }
        }
        other => panic!("expected Unbalanced/UnexpectedCloser for extra `}}`, got {other:?}"),
    }
}

/// Bare-record lookback must ignore `key: value}` inside raw strings.
#[test]
fn parse_record_pattern_inside_raw_string_does_not_reshape() {
    let file = b"\
def f [] {\n\
  let s = r#'first\n\
type: text}\n\
last'#\n\
}\n\
}\n";
    let err = parse_first_error(file);
    match &err {
        ParseError::Unbalanced(_, close, close_span, _) => {
            assert_eq!(*close, "}");
            assert_eq!(file[close_span.start], b'}');
            assert_eq!(line_of(file, close_span.start), 6);
        }
        ParseError::UnexpectedCloser {
            closer,
            closer_span,
            hint_span,
            hint_label,
            ..
        } => {
            assert_eq!(*closer, "}");
            assert_eq!(file[closer_span.start], b'}');
            if let Some(hint) = hint_span {
                assert_ne!(
                    line_of(file, hint.start),
                    3,
                    "must not label raw-string content as record insert site: {hint_label}"
                );
            }
        }
        other => panic!("expected Unbalanced/UnexpectedCloser for extra `}}`, got {other:?}"),
    }
}

/// Same string-content guard for double-quoted multiline strings.
#[test]
fn parse_if_inside_multiline_double_quoted_string_does_not_reshape() {
    let file = b"\
def f [] {\n\
  let s = \"first\n\
if this is just string content\n\
last\"\n\
}\n\
}\n";
    let err = parse_first_error(file);
    match &err {
        ParseError::Unbalanced(_, close, close_span, _) => {
            assert_eq!(*close, "}");
            assert_eq!(file[close_span.start], b'}');
        }
        ParseError::UnexpectedCloser {
            closer,
            closer_span,
            hint_span,
            ..
        } => {
            assert_eq!(*closer, "}");
            assert_eq!(file[closer_span.start], b'}');
            if let Some(hint) = hint_span {
                assert_ne!(line_of(file, hint.start), 3);
            }
        }
        other => panic!("expected Unbalanced/UnexpectedCloser, got {other:?}"),
    }
}

#[test]
fn lex_mismatched_closer_list_closed_with_paren() {
    // `)` closing a `[` should mention `[`, not invent `(` as the open kind,
    // and must not fire the missing-`(` presentation.
    let file = b"[1, 2, 3)";
    let err = lex(file, 0, &[], &[], true)
        .1
        .expect("expected unbalanced error");
    assert_unbalanced(&err, "[", ")");
}

#[test]
fn lex_mismatched_closer_paren_closed_with_bracket() {
    // `]` closing a `(` must keep unbalanced-with-`(`, not missing-`[`.
    let file = b"(1, 2]";
    let err = lex(file, 0, &[], &[], true)
        .1
        .expect("expected unbalanced error");
    assert_unbalanced(&err, "(", "]");
}

#[test]
fn lex_mismatched_closer_brace_closed_with_paren() {
    // Wrong closer for a record/block. Presentation may reshape to unexpected-`)`,
    // but must still be a real stack failure (never silent).
    let file = b"{ a: 1 )";
    let err = lex(file, 0, &[], &[], true)
        .1
        .expect("expected delimiter error");
    match &err {
        ParseError::Unbalanced(open, close, ..) => {
            assert_eq!(*close, ")");
            assert_eq!(*open, "{");
        }
        ParseError::UnexpectedCloser { closer, help, .. } => {
            assert_eq!(*closer, ")");
            assert!(
                help.contains("if it is extra") && help.contains("or add"),
                "help should be dual remove/add, got: {help}"
            );
        }
        other => panic!("expected Unbalanced or UnexpectedCloser, got {other:?}"),
    }
}

#[test]
fn lex_mismatched_bracket_inside_block_with_sig_brackets() {
    // `def f [] { 1 ] }` has `[` earlier on the line (empty signature). Must not
    // claim "missing `[`" for the list — report unbalanced against `{`.
    let file = b"def f [] { 1 ] }";
    let err = parse_first_error(file);
    assert_unbalanced(&err, "{", "]");
}

#[test]
fn parse_missing_open_paren_points_at_grouped_expr() {
    // `print -n ansi green)` — likely insert site before `ansi` (defs.nu bar pattern).
    // Message stays dual-path: remove `)` or add `(`. Primary on `)`.
    let file = b"def f [] {\n  if $x {\n        print -n ansi green)\n  }\n}\n";
    let err = parse_first_error(file);
    let (closer, hint) = assert_unexpected_closer(&err, ")", "possible place for `(`");
    assert_eq!(
        file[closer.start], b')',
        "primary must be the unmatched `)`"
    );
    assert_eq!(
        snippet_at(file, hint).chars().next(),
        Some('a'),
        "should point near `ansi`, got {:?}",
        snippet_at(file, hint)
    );
}

#[test]
fn parse_extra_paren_help_is_dual_remove_or_add() {
    // Review cases: remove is often correct, but help must not only say "add `(`".
    for file in [b"1 + 2)" as &[u8], b"let x = 1)", b"{ a: 1 ) }"] {
        let err = parse_first_error(file);
        match &err {
            ParseError::Unbalanced(_, close, _, help) => {
                assert_eq!(*close, ")");
                assert!(
                    help.contains("if it is extra") && help.contains("or add"),
                    "unbalanced help should be dual remove/add, got: {help}"
                );
            }
            ParseError::UnexpectedCloser {
                closer,
                help,
                closer_span,
                ..
            } => {
                assert_eq!(*closer, ")");
                assert_eq!(file[closer_span.start], b')');
                assert!(
                    help.contains("if it is extra") && help.contains("or add"),
                    "unexpected-closer help should be dual remove/add, got: {help}"
                );
            }
            other => panic!("expected Unbalanced or UnexpectedCloser for {file:?}, got {other:?}"),
        }
    }
}

#[test]
fn parse_balanced_parens_with_extra_close_stays_unbalanced() {
    // `print (ansi green))` has a balanced group then an extra `)` — must not
    // reshape into insert-site labeling (line already has `(`).
    let file = b"def f [] { print (ansi green)) }\n";
    let err = parse_first_error(file);
    match &err {
        ParseError::Unbalanced(open, close, ..) => {
            assert_eq!(*close, ")");
            assert!(
                *open == "(" || *open == "{",
                "unexpected open kind {open:?} in {err:?}"
            );
        }
        ParseError::UnexpectedCloser {
            hint_span: Some(_), ..
        } => {
            panic!("extra `)` must not reshape to insert-site label: {err:?}");
        }
        other => panic!("expected Unbalanced for extra `)`, got {other:?}"),
    }
}

#[test]
fn parse_try_identifier_prefix_not_missing_brace() {
    // `try_this` must not match the bare-`try` missing-`{` lookback.
    let file = b"def f [] {\n  try_this\n  1\n}\n}\n";
    let err = parse_first_error(file);
    assert_unbalanced(&err, "{", "}");
}

#[test]
fn parse_missing_list_open_bracket_points_at_list_start() {
    // `2 (x - 2) 0]` inside an if block — likely insert site at list start.
    let file = b"def f [] {\n  if $x {\n      2 ($in_ten - 2) 0]\n  }\n}\n";
    let err = parse_first_error(file);
    let (closer, hint) = assert_unexpected_closer(&err, "]", "possible place for `[`");
    assert_eq!(
        file[closer.start], b']',
        "primary must be the unmatched `]`"
    );
    assert_eq!(
        snippet_at(file, hint).chars().next(),
        Some('2'),
        "should point near list start `2`, got {:?}",
        snippet_at(file, hint)
    );
}

#[test]
fn parse_missing_sig_close_before_body_brace() {
    // `def name [\n  param\n {` — missing `]` before body; label at body `{`.
    let file = b"def prepend-if-not-in [\n  value: string\n {\n  let list = $in\n}\n";
    let err = parse_first_error(file);
    let (open, expected) = assert_unclosed(&err, "]");
    assert_eq!(file[open.start], b'[');
    assert_eq!(file[expected.start], b'{');
    assert_eq!(line_of(file, expected.start), 3);
}

#[test]
fn parse_unclosed_quotes_still_report() {
    let file = b"let x = \"hello";
    let err = parse_first_error(file);
    // Quote failures may surface as Unclosed("\"") or similar string errors.
    let msg = format!("{err:?}");
    assert!(
        matches!(err, ParseError::Unclosed(d, ..) if d.contains('"') || d.contains('\''))
            || msg.to_lowercase().contains("quote")
            || msg.contains('\"'),
        "expected quote-related error, got {err:?}"
    );
}

#[rstest]
#[case::single_line(b"let x = \"hello")]
#[case::multiline(b"let x = \"hello\nworld")]
#[case::multiline_single_quotes(b"let x = 'hello\nworld")]
fn parse_unclosed_quotes_multiline_still_report(#[case] file: &[u8]) {
    // Multi-line string content is fine only when closed; missing closer is Unclosed.
    let err = parse_first_error(file);
    let msg = format!("{err:?}");
    assert!(
        matches!(err, ParseError::Unclosed(d, ..) if d.contains('"') || d.contains('\''))
            || msg.to_lowercase().contains("quote")
            || msg.contains('\"')
            || msg.contains('\''),
        "expected unclosed quote for {:?}, got {err:?}",
        std::str::from_utf8(file)
    );
}

#[test]
fn parse_multiline_unclosed_list_reports_unclosed() {
    // Same shape as a valid multi-line list, but without the closing `]`.
    let file = b"let y = [1, 2, 3\n4, 5, 6";
    let err = parse_first_error(file);
    let (open, _) = assert_unclosed(&err, "]");
    assert_eq!(file[open.start], b'[');
}

/// `}` and `)` always diagnose on empty stack. Bare `]` is not treated as a
/// closer when nothing is open (it can be an ordinary item character in some
/// positions), so it is intentionally omitted here.
#[rstest]
#[case::extra_brace(b"}", "}", "{")]
#[case::extra_paren(b")", ")", "(")]
fn lex_extra_closers_on_empty_stack(
    #[case] src: &[u8],
    #[case] close: &str,
    #[case] default_open: &str,
) {
    let err = lex(src, 0, &[], &[], true)
        .1
        .expect("expected unbalanced closer on empty stack");
    // Empty stack: unbalanced with the default opener for that closer.
    // (Missing-open presentation only kicks in with lookback context.)
    assert_unbalanced(&err, default_open, close);
}

#[test]
fn lex_comments_no_space() {
    // test for parses that contain tokens that normally introduce comments
    // Code:
    // let z = 42 #the comment
    // let x#y = 69 #hello
    // let flk = nixpkgs#hello #hello
    let file = b"let z = 42 #the comment \n let x#y = 69 #hello \n let flk = nixpkgs#hello #hello";
    let output = lex(file, 0, &[], &[], false);

    assert_eq!(
        output.0.get(4).unwrap(),
        &Token {
            contents: TokenContents::Comment,
            span: Span::new(11, 24)
        }
    );

    assert_eq!(
        output.0.get(7).unwrap(),
        &Token {
            contents: TokenContents::Item,
            span: Span::new(30, 33)
        }
    );

    assert_eq!(
        output.0.get(10).unwrap(),
        &Token {
            contents: TokenContents::Comment,
            span: Span::new(39, 46)
        }
    );

    assert_eq!(
        output.0.get(15).unwrap(),
        &Token {
            contents: TokenContents::Item,
            span: Span::new(58, 71)
        }
    );

    assert_eq!(
        output.0.get(16).unwrap(),
        &Token {
            contents: TokenContents::Comment,
            span: Span::new(72, 78)
        }
    );
}

#[test]
fn lex_comments() {
    // Comments should keep the end of line token
    // Code:
    // let z = 4
    // let x = 4 #comment
    // let y = 1 # comment
    let file = b"let z = 4 #comment \n let x = 4 # comment\n let y = 1 # comment";

    let output = lex(file, 0, &[], &[], false);

    assert_eq!(
        output.0.get(4).unwrap(),
        &Token {
            contents: TokenContents::Comment,
            span: Span::new(10, 19)
        }
    );
    assert_eq!(
        output.0.get(5).unwrap(),
        &Token {
            contents: TokenContents::Eol,
            span: Span::new(19, 20)
        }
    );

    // When there is no space between the comment and the new line the span
    // for the command and the EOL overlaps
    assert_eq!(
        output.0.get(10).unwrap(),
        &Token {
            contents: TokenContents::Comment,
            span: Span::new(31, 40)
        }
    );
    assert_eq!(
        output.0.get(11).unwrap(),
        &Token {
            contents: TokenContents::Eol,
            span: Span::new(40, 41)
        }
    );
}

#[test]
fn lex_manually() {
    let file = b"'a'\n#comment\n#comment again\n| continue";
    let mut lex_state = LexState {
        input: file,
        output: Vec::new(),
        error: None,
        span_offset: 10,
    };
    assert_eq!(lex_n_tokens(&mut lex_state, &[], &[], false, 1), 1);
    assert_eq!(lex_state.output.len(), 1);
    assert_eq!(lex_n_tokens(&mut lex_state, &[], &[], false, 5), 5);
    assert_eq!(lex_state.output.len(), 6);
    // Next token is the pipe.
    // This shortens the output because it exhausts the input before it can
    // compensate for the EOL tokens lost to the line continuation
    assert_eq!(lex_n_tokens(&mut lex_state, &[], &[], false, 1), -1);
    assert_eq!(lex_state.output.len(), 5);
    assert_eq!(file.len(), lex_state.span_offset - 10);
    let last_span = lex_state.output.last().unwrap().span;
    assert_eq!(&file[last_span.start - 10..last_span.end - 10], b"continue");
}

/// Large nested record must lex without error (regression for O(n²) indent
/// heuristics). Wall-clock bounds are intentionally not asserted — they flake
/// under CI load / debug builds.
#[test]
fn lex_large_nested_record_completes() {
    let mut src = String::from("$env.config = {\n");
    for i in 0..2000 {
        let _ = write!(src, "  key{i}: {{\n    nested: {i}\n  }}\n");
    }
    src.push('}');
    let (_tokens, err) = lex(src.as_bytes(), 0, &[], &[], true);
    assert!(err.is_none(), "unexpected lex error: {err:?}");
}

#[test]
fn parse_bare_string_interpolation_with_two_paren_groups() {
    // Regression: Missing-`(` presentation reshapes some `)` failures. That must
    // not block the paren-expr fallback to bare-word string interpolation
    // (e.g. `(100 + 20 + 3)/bar/(300 + 20 + 1)`).
    let file = b"(100 + 20 + 3)/bar/(300 + 20 + 1)";
    let engine = nu_protocol::engine::EngineState::new();
    let mut ws = nu_protocol::engine::StateWorkingSet::new(&engine);
    let block = nu_parser::parse(&mut ws, None, file, true);
    assert!(
        ws.parse_errors.is_empty(),
        "bare interpolation must parse cleanly, got {:?}",
        ws.parse_errors
    );
    let pipeline = &block.pipelines[0];
    let expr = &pipeline.elements[0].expr.expr;
    assert!(
        matches!(expr, nu_protocol::ast::Expr::StringInterpolation(_)),
        "expected StringInterpolation, got {expr:?}"
    );
}
