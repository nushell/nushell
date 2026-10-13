//! Regression tests for the `winnow-parser` front end (literals): each pins a case where it
//! parsed differently from the classic parser.

use nu_test_support::prelude::*;
use rstest::rstest;

/// A raw string whose closing `#`s are followed by a multi-byte character is unbalanced, as the
/// classic parser reports; the front end used to panic cutting the item inside that character.
#[test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn raw_string_followed_by_a_multibyte_character() -> Result {
    test()
        .run("r#'a'#\u{e9}")
        .expect_error_code_eq("nu::parser::unbalanced_delimiter")
}

/// The quote that opens a raw string may close it, as in nu's lexer: `r#'#foo'#` is unclosed.
#[test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn raw_string_opening_quote_may_close_it() -> Result {
    test()
        .run("[r#'#foo'#]")
        .expect_error_code_eq("nu::parser::unclosed_delimiter")
}

/// A `#` after a vertical tab starts a comment, so the `]` after it does not close the list.
#[test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn hash_after_a_vertical_tab_starts_a_comment() -> Result {
    test().run("[1\u{b}# x ]\n 2] | length").expect_value_eq(4)
}

/// In a `where` condition a raw string is a value, not a column of the row.
#[test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn raw_string_in_a_row_condition_is_a_value() -> Result {
    test()
        .run("[{a: 1} {a: 2}] | where r#'a'# == 'a' | length")
        .expect_value_eq(2)
}

/// A range whose text ends with its operator has no upper bound: `1...` is `1..`.
#[test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn range_ending_in_three_dots_is_open() -> Result {
    test()
        .run("let x = 1...; $x | first 3 | to nuon")
        .expect_value_eq("[1, 2, 3]")
}

/// The second default of a parameter without a type is parsed with the first default's type.
#[test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn second_default_takes_the_first_defaults_type() -> Result {
    test()
        .run("def f [x = a = 1] { $x | describe }; f")
        .expect_value_eq("string")
}

/// `1..=5` lexes as `1..`, `=`, `5` in a signature, and `5` is no range.
#[test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn second_default_that_is_not_of_the_first_defaults_type() -> Result {
    test()
        .run("def f [x = 1..=5] { $x | describe }; f")
        .expect_error_code_eq("nu::parser::parse_mismatch")
}

/// A datetime offset may use U+2212 for its minus, as chrono's RFC 3339 parser accepts.
#[test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn datetime_offset_with_a_unicode_minus() -> Result {
    test()
        .run("2024-01-02T03:04:05\u{2212}05:00 | describe")
        .expect_value_eq("datetime")
}

/// Ranges the classic parser commits to and refuses, in a value and at the head of a command.
#[rstest]
#[case::record_bound("[5..{a:1}]", "nu::parser::operator_unsupported_type")]
#[case::record_bound_head("5..{a:1}", "nu::parser::operator_unsupported_type")]
#[case::raw_string_bound_head("let x = 5..r#'a'#", "nu::parser::operator_unsupported_type")]
#[case::cell_path_literal_bound_head("let y = 1..$.a", "nu::parser::operator_unsupported_type")]
#[case::empty_next("[1....5]", "nu::parser::parser_incomplete")]
#[nu_test_support::test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn ranges_the_classic_parser_refuses(#[case] code: &str, #[case] error_code: &str) -> Result {
    test().run(code).expect_error_code_eq(error_code)
}

/// A radix prefix after a `_` separator still makes the word an int, with invalid digits.
#[test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn radix_prefix_after_an_underscore() -> Result {
    let error = test().run("[0_xzz]").expect_parse_error()?;
    assert!(matches!(error, ParseError::InvalidLiteral(..)), "{error:?}");
    Ok(())
}

/// `if`, `while`, `return`, ... are commands for the classic parser: a longer command name that
/// starts with the keyword is called instead.
#[rstest]
#[case::if_ready(r#"def "if ready" [then: closure] { do $then }; if ready { "ran" }"#)]
#[case::while_ready(r#"def "while ready" [then: closure] { do $then }; while ready { "ran" }"#)]
#[case::return_me(r#"def "return me" [] { "ran" }; def g [] { return me }; g"#)]
#[nu_test_support::test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn keyword_that_starts_a_longer_command_name(#[case] code: &str) -> Result {
    test().run(code).expect_value_eq("ran")
}
