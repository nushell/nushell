use crate::repl::tests::{TestResult, fail_test, run_test};
use nu_test_support::prelude::*;
use pretty_assertions::assert_matches;
use rstest::rstest;

#[test]
fn cjk_in_substrings() -> TestResult {
    run_test(
        "let s = '[Rust 程序设计语言](title-page.md)'; let start = ($s | str index-of '('); let end = ($s | str index-of ')'); $s | str substring ($start + 1)..<($end)",
        "title-page.md",
    )
}

#[test]
fn string_not_in_string() -> TestResult {
    run_test("'d' not-in 'abc'", "true")
}

#[test]
fn string_in_string() -> TestResult {
    run_test("'z' in 'abc'", "false")
}

#[test]
fn non_string_in_string() -> TestResult {
    fail_test("42 in 'abc'", "nu::parser::operator_incompatible_types")
}

#[test]
fn string_in_record() -> TestResult {
    run_test(r#""a" in ('{ "a": 13, "b": 14 }' | from json)"#, "true")
}

#[test]
fn non_string_in_record() -> TestResult {
    fail_test(
        r#"4 in ('{ "a": 13, "b": 14 }' | from json)"#,
        "nu::shell::operator_incompatible_types",
    )
}

#[test]
fn unbalance_string() -> TestResult {
    fail_test(r#""aaaab"cc"#, "invalid characters")?;
    fail_test("'aaaab'cc", "invalid characters")?;
    fail_test(r#"$"aaaab"cc"#, "invalid characters")?;
    fail_test("$'aaaab'cc", "invalid characters")
}

#[test]
fn string_in_valuestream() -> TestResult {
    run_test(
        r#"
    'Hello' in ("Hello
    World" | lines)"#,
        "true",
    )
}

#[test]
fn single_tick_interpolation() -> TestResult {
    run_test("$'(3 + 4)'", "7")
}

#[test]
fn unclosed_interpolation_subexpression() -> TestResult {
    fail_test("$\"foo (2 + 3\"", "expected `)`")
}

#[test]
fn detect_newlines() -> TestResult {
    run_test("'hello\r\nworld' | lines | get 0 | str length", "5")
}

#[test]
fn case_insensitive_sort() -> TestResult {
    run_test(
        "[a, B, d, C, f] | sort -i | to json --raw",
        "[\"a\",\"B\",\"C\",\"d\",\"f\"]",
    )
}

#[test]
fn case_insensitive_sort_columns() -> TestResult {
    run_test(
        r#"[[version, package]; ["two", "Abc"], ["three", "abc"], ["four", "abc"]] | sort-by -i package version | to json --raw"#,
        r#"[{"version":"four","package":"abc"},{"version":"three","package":"abc"},{"version":"two","package":"Abc"}]"#,
    )
}

#[test]
fn raw_string() -> TestResult {
    run_test(r#"r#'abcde""fghi"''''jkl'#"#, r#"abcde""fghi"''''jkl"#)?;
    run_test(r#"r##'abcde""fghi"''''#jkl'##"#, r#"abcde""fghi"''''#jkl"#)?;
    run_test(
        r#"r###'abcde""fghi"'''##'#jkl'###"#,
        r#"abcde""fghi"'''##'#jkl"#,
    )?;
    run_test("r#''#", "")?;
    run_test(
        "r#'a string with sharp inside # and ends with #'#",
        "a string with sharp inside # and ends with #",
    )
}

#[test]
fn raw_string_inside_parentheses() -> TestResult {
    let (left, right) = ('(', ')');
    run_test(
        &format!(r#"{left}r#'abcde""fghi"''''jkl'#{right}"#),
        r#"abcde""fghi"''''jkl"#,
    )?;
    run_test(
        &format!(r#"{left}r##'abcde""fghi"''''#jkl'##{right}"#),
        r#"abcde""fghi"''''#jkl"#,
    )?;
    run_test(
        &format!(r#"{left}r###'abcde""fghi"'''##'#jkl'###{right}"#),
        r#"abcde""fghi"'''##'#jkl"#,
    )?;
    run_test(&format!("{left}r#''#{right}"), "")?;
    run_test(
        &format!("{left}r#'a string with sharp inside # and ends with #'#{right}"),
        "a string with sharp inside # and ends with #",
    )
}

#[test]
fn raw_string_inside_list() -> TestResult {
    let (left, right) = ('[', ']');
    run_test(
        &format!(r#"{left}r#'abcde""fghi"''''jkl'#{right} | get 0"#),
        r#"abcde""fghi"''''jkl"#,
    )?;
    run_test(
        &format!(r#"{left}r##'abcde""fghi"''''#jkl'##{right} | get 0"#),
        r#"abcde""fghi"''''#jkl"#,
    )?;
    run_test(
        &format!(r#"{left}r###'abcde""fghi"'''##'#jkl'###{right} | get 0"#),
        r#"abcde""fghi"'''##'#jkl"#,
    )?;
    run_test(&format!("{left}r#''#{right} | get 0"), "")?;
    run_test(
        &format!("{left}r#'a string with sharp inside # and ends with #'#{right} | get 0"),
        "a string with sharp inside # and ends with #",
    )
}

#[test]
fn raw_string_inside_closure() -> TestResult {
    let (left, right) = ('{', '}');
    run_test(
        &format!(r#"do {left}r#'abcde""fghi"''''jkl'#{right}"#),
        r#"abcde""fghi"''''jkl"#,
    )?;
    run_test(
        &format!(r#"do {left}r##'abcde""fghi"''''#jkl'##{right}"#),
        r#"abcde""fghi"''''#jkl"#,
    )?;
    run_test(
        &format!(r#"do {left}r###'abcde""fghi"'''##'#jkl'###{right}"#),
        r#"abcde""fghi"'''##'#jkl"#,
    )?;
    run_test(&format!("do {left}r#''#{right}"), "")?;
    run_test(
        &format!("do {left}r#'a string with sharp inside # and ends with #'#{right}"),
        "a string with sharp inside # and ends with #",
    )
}

#[test]
fn incomplete_string() -> TestResult {
    fail_test("r#abc", "expected '")?;
    fail_test("r#'bc", "expected `'#`")?;
    fail_test("'ab\"", "expected `'`")?;
    fail_test("\"ab'", "expected `\"`")?;
    fail_test(
        r#"def func [] {
  {
    "A": ""B"   # <- the quote is bad
  }
}
"#,
        "expected `\"`",
    )
}

/// Text after a string's last quote is an error.
#[rstest]
#[case::single_then_bare("'a'b")]
#[case::single_then_double(r#"'a'"b""#)]
fn text_after_closing_quote(#[case] code: &str) -> Result {
    test()
        .run(code)
        .expect_error_code_eq("nu::parser::extra_token_after_closing_delimiter")
}

/// A quoted word runs to its last quote of the same kind, and the quotes of that kind inside it
/// are part of the string. Scripts rely on this for JSON and HTML text.
#[rstest]
#[case::single("'a''b'", "a''b")]
#[case::double(r#""[{a: "b"}]""#, r#"[{a: "b"}]"#)]
#[case::interpolation(r#"let c = 'x'; $"<p class="($c)">""#, r#"<p class="x">"#)]
fn same_quotes_inside_a_string(#[case] code: &str, #[case] expected: &str) -> Result {
    test().run(code).expect_value_eq(expected)
}

/// An escaped quote and a quote inside an interpolated subexpression don't close the string.
#[rstest]
#[case::escaped_quote(r#""a\"b""#, "a\"b")]
#[case::subexpression_quote(r#"$"a(")")b""#, "a)b")]
fn quotes_that_do_not_close(#[case] code: &str, #[case] expected: &str) -> Result {
    test().run(code).expect_value_eq(expected)
}

/// `\x` escapes add raw bytes, which must form valid UTF-8.
#[test]
fn hex_escapes_invalid_utf8() -> Result {
    let err = test().run(r#""\xFF""#).expect_parse_error()?;
    assert_matches!(err, ParseError::InvalidLiteral(msg, ..) if msg.contains("UTF-8"));
    test().run(r#""\xC3\xA9""#).expect_value_eq("é")
}

/// An unclosed raw string reports the closer it needs, like any other unclosed string.
#[rstest]
#[case::one_hash("r#'abc", "'#")]
#[case::two_hashes("r##'abc", "'##")]
fn unclosed_raw_string(#[case] code: &str, #[case] closer: &str) -> Result {
    let err = test().run(code).expect_parse_error()?;
    assert_matches!(err, ParseError::Unclosed(delimiter, ..) if delimiter == closer);
    Ok(())
}
