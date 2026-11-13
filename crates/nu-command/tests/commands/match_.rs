use pretty_assertions::assert_matches;
use rstest::rstest;

use nu_test_support::prelude::*;

#[rstest]
#[case::inside(3, true)]
#[case::outside(11, false)]
fn range(#[case] scrutinee: impl IntoValue, #[case] success: bool) -> Result {
    let code = "
        match $in {
            1..10 => { true }
            _ => { false }
        }
    ";
    test()
        .run_with_data(code, scrutinee)
        .expect_value_eq(success)
}

#[test]
fn record() -> Result {
    test()
        .run(
            "
            match {a: 11} {
                {a: $b} => { $b }
            }
            ",
        )
        .expect_value_eq(11)
}

#[test]
fn record_shorthand() -> Result {
    test()
        .run(
            "
            match {a: 12} {
                {$a} => { $a }
            }
            ",
        )
        .expect_value_eq(12)
}

#[test]
fn list() -> Result {
    test()
        .run(
            "
            match [1, 2] {
                [$a] => { {single: $a} }
                [$b, $c] => { {double: [$b, $c] } }
            }
            ",
        )
        .expect_value_eq(test_value!({double: [1, 2]}))
}

#[test]
fn list_rest_ignore() -> Result {
    test()
        .run(
            "
            match [1, 2] {
                [$a, ..] => { {single: $a} }
                [$b, $c] => { {double: [$b, $c] } }
            }
            ",
        )
        .expect_value_eq(test_value!({single: 1}))
}

#[test]
fn list_rest_capture() -> Result {
    test()
        .run(
            "
                match [1, 2, 3] {
                    [$a, ..$remainder] => { {single: $a, remainder: $remainder} }
                    [$b, $c] => { {double: [$b, $c]} }
                }
            ",
        )
        .expect_value_eq(test_value!({
            single: 1,
            remainder: [2, 3],
        }))
}

#[test]
fn list_rest_empty() -> Result {
    test()
        .run("match [1] { [1 ..$rest] => { $rest == [] } }")
        .expect_value_eq(true)
}

#[rstest]
#[case::at_start("match [2] { [1, ..] => false, [2, ..] => true }")]
#[case::at_end("match [2] { [.., 1] => false, [.., 2] => true }")]
#[case::in_middle("match [1 2 3] { [1, .., 2] => false, [1, .., 3] => true }")]
#[case::empty_in_middle("match [1 2] { [1, .., 2] => true }")]
#[case::list_too_short("match [1 2] { [1, .., 1, 2] => false, _ => true }")]
#[case::list_too_short_capture("match [1 2] { [1, ..$rest, 1, 2] => false, _ => true }")]
#[case::empty_list("match [] { [.., 2] => false, _ => true }")]
#[case::nested("match [[1 2] 3] { [[.., 1], ..] => false, [[.., 2], ..] => true }")]
fn list_rest_position(#[case] code: &str) -> Result {
    test().run(code).expect_value_eq(true)
}

#[rstest]
#[case::in_middle(
    "match [1 2 3 4] { [$a, ..$rest, $b] => [$a $rest $b] }",
    test_value!([1, [2, 3], 4])
)]
#[case::empty_in_middle(
    "match [1 2] { [$a, ..$rest, $b] => [$a $rest $b] }",
    test_value!([1, [], 2])
)]
#[case::at_start(
    "match [1 2 3] { [..$rest, $b] => [$rest $b] }",
    test_value!([[1, 2], 3])
)]
fn list_rest_capture_position(#[case] code: &str, #[case] expected: Value) -> Result {
    test().run(code).expect_value_eq(expected)
}

#[rstest]
#[case::ignore("match [2] { [.., 2, ..] => true }")]
#[case::capture("match [2] { [..$a, 2, ..$b] => true }")]
#[case::mixed("match [2] { [..$a, 2, ..] => true }")]
fn list_rest_used_twice(#[case] code: &str) -> Result {
    let err = test().run(code).expect_parse_error()?;
    assert_contains("can only be used once per list pattern", err.to_string());
    Ok(())
}

#[rstest]
#[case::int(["1", "2", "3"])]
#[case::float(["1.4", "2.3", "3"])]
#[case::bool(["false", "true", "3"])]
#[case::string(["\"abc\"", "\"def\"", "\"ghi\""])]
#[case::date(["2010-01-01", "2019-08-23", "2020-02-02"])]
#[case::duration(["2sec", "6sec", "1min"])]
#[case::filesize(["1kb", "1kib", "2kb"])]
fn literal(#[case] patterns: [&str; 3]) -> Result {
    let [first, second, third] = patterns;

    let mut tester = test();
    let scrutinee: Value = tester.run(second)?;

    let code = indoc::formatdoc! {"
        match $in {{
            {first} => false,
            {second} => true,
            {third} => false,
        }}
    "};
    test().run_with_data(code, scrutinee).expect_value_eq(true)
}

#[test]
fn literal_raw_string() -> Result {
    test()
        .run(
            r#"
            match "foo" {
                r#'foo'# => true,
                _ => false,
            }
        "#,
        )
        .expect_value_eq(true)
}

#[test]
fn literal_null() -> Result {
    test()
        .run(
            "
            match null {
                null => true,
                _ => false,
            }
        ",
        )
        .expect_value_eq(true)
}

#[test]
fn match_or_pattern() -> Result {
    test()
        .run(
            "
        match {b: 7} {
            {a: $a} | {b: $b} => { {success: $b} }
            _ => false
        }
    ",
        )
        .expect_value_eq(test_value!({success: 7}))
}

#[test]
fn match_or_pattern_overlap_1() -> Result {
    test()
        .run(
            "
        match {a: 7} {
            {a: $b} | {b: $b} => { {success: $b} }
            _ => false
        }
    ",
        )
        .expect_value_eq(test_value!({success: 7}))
}

#[test]
fn match_or_pattern_overlap_2() -> Result {
    test()
        .run(
            r#"
        match {b: 7} {
            {a: $b} | {b: $b} => { {success: $b} }
            _ => { print "failure" }
        }
    "#,
        )
        .expect_value_eq(test_value!({success: 7}))
}

#[test]
fn match_doesnt_overwrite_variable() -> Result {
    test()
        .run("let b = 100; match 55 { $b => {} }; $b")
        .expect_value_eq(100)
}

#[test]
fn match_with_guard() -> Result {
    test()
        .run(
            "
        match [1 2 3] {
            [$x, ..] if $x mod 2 == 0 => false,
            $x => true,
        }
    ",
        )
        .expect_value_eq(true)
}

#[test]
fn match_with_guard_block_as_guard() -> Result {
    // this should work?
    test()
        .run("match 4 { $x if { $x + 20 > 25 } => { 'good num' }, _ => { 'terrible num' } }")
        .expect_error_code_eq("nu::shell::match_guard_not_bool")
}

#[test]
fn match_with_guard_parens_expr_as_guard() -> Result {
    test()
        .run("match 4 { $x if ($x + 20 > 25) => { 'good num' }, _ => { 'terrible num' } }")
        .expect_value_eq("terrible num")
}

#[test]
fn match_with_guard_not_bool() -> Result {
    test()
        .run("match 4 { $x if $x + 1 => { 'err!()' }, _ => { 'unreachable!()' } }")
        .expect_error_code_eq("nu::shell::match_guard_not_bool")
}

#[test]
fn match_with_guard_no_expr_after_if() -> Result {
    let err = test()
        .run("match 4 { $x if  => { 'err!()' }, _ => { 'unreachable!()' } }")
        .expect_parse_error()?;
    assert_contains("Match guard without an expression", err.to_string());
    Ok(())
}

#[test]
fn match_with_guard_multiarm() -> Result {
    test()
        .run("match 3 {1 | 2 | 3 if true => 'test'}")
        .expect_value_eq("test")
}

#[test]
fn match_with_or_missing_expr() -> Result {
    let err = test().run("match $in { 1 | }").expect_parse_error()?;
    assert_matches!(
        err,
        ParseError::Mismatch(expected, found, _) if expected == "pattern" && found == "end of input"
    );
    Ok(())
}

#[test]
fn line_comment_in_match_block() -> Result {
    let code = "
        match 1 {
            # comment
            _ => { true }
        }
    ";
    test().run(code).expect_value_eq(true)
}

#[test]
fn trailing_comment_after_match_arm() -> Result {
    let code = "
        match 1 {
            _ => { true } # comment
        }
    ";
    test().run(code).expect_value_eq(true)
}

#[rstest]
#[case::string_concat("match 'test' { ( 't' + 'es' + 't' ) => { true } }")]
#[case::int_arithmetic("match 42 { (40 + 2) => { true } }")]
#[case::no_match("match 'nope' { ('t' + 'es' + 't') => { false }, _ => { true } }")]
#[case::range_literal("match 5 { (1..10) => { true }, _ => { false } }")]
#[case::or_pattern("match 3 { (1 + 1) | 3 => { true }, _ => { false } }")]
#[case::nested_in_list("match [2] { [(1 + 1)] => { true }, _ => { false } }")]
fn const_expr_in_paren(#[case] code: &str) -> Result {
    test().run(code).expect_value_eq(true)
}

#[test]
fn const_expr_with_const_var() -> Result {
    test()
        .run(
            r#"
            const BASE = "/Users/fdncred"
            match "/Users/fdncred/src/nushell" {
                ($BASE + "/other") => { "other" },
                ($BASE + "/src/nushell") => { "nu" },
                _ => { "miss" }
            }
        "#,
        )
        .expect_value_eq("nu")
}

#[test]
fn match_paren_expression_non_const_errors() -> Result {
    let err = test()
        .run(
            r#"
            match 'x' {
                ($env.HOME) => { "FAIL" }
                _ => { "OK" }
            }
        "#,
        )
        .expect_parse_error()?;

    assert_contains("not a parse-time constant", err.to_string());
    Ok(())
}
