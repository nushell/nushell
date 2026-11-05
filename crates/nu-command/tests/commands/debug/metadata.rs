use nu_test_support::prelude::*;
use rstest::rstest;

// See `Metadata::run` for why the argument wins over block input.
#[rstest]
#[case::closure_parameter(
    "[1] | each {|x| let s = (metadata $x).span; view span $s.start $s.end } | first",
    "1"
)]
#[case::literal_in_block(
    "'a' | do { let s = (metadata 5).span; view span $s.start $s.end }",
    "5"
)]
#[case::in_in_block(
    "'a' | do { let s = (metadata $in).span; view span $s.start $s.end }",
    "'a'"
)]
#[case::in_piped("let s = ('a' | metadata $in).span; view span $s.start $s.end", "'a'")]
fn argument_wins_over_input(#[case] code: &str, #[case] expected: &str) -> Result {
    test().run(code).expect_value_eq(expected)
}

// `null` must still count as an argument, or the input's `content_type` would show up.
#[rstest]
#[case::literal("5")]
#[case::null("null")]
fn argument_ignores_input_metadata(#[case] arg: &str) -> Result {
    let code = format!(
        "
        'x'
        | metadata set --content-type text/plain
        | do {{ let m = (metadata {arg}); $m | columns }}
        "
    );

    test().run(&code).expect_value_eq(["span"])
}

#[test]
fn no_argument_reads_block_input() -> Result {
    let code = "
        'x'
        | metadata set --content-type text/plain
        | do { metadata | get content_type }
    ";

    test().run(code).expect_value_eq("text/plain")
}

// A variable captured by a closure keeps the span it was defined with.
#[rstest]
#[case::closure("let x = 'abc'; do { let s = (metadata $x).span; view span $s.start $s.end }")]
#[case::nested_closure(
    "let x = 'abc'; do { do { let s = (metadata $x).span; view span $s.start $s.end } }"
)]
#[case::each(
    "let x = 'abc'; [1] | each { let s = (metadata $x).span; view span $s.start $s.end } | first"
)]
#[case::closure_in_def(
    "let x = 'abc'; def f [] { do { let s = (metadata $x).span; view span $s.start $s.end } }; f"
)]
#[case::constant("const x = 'abc'; do { let s = (metadata $x).span; view span $s.start $s.end }")]
fn captured_variable_keeps_definition_span(#[case] code: &str) -> Result {
    test().run(code).expect_value_eq("'abc'")
}

#[test]
fn captured_argument_keeps_definition_span() -> Result {
    let code = "
        def f [arg] { do { let s = (metadata $arg).span; view span $s.start $s.end } }
        f foobar
    ";
    test().run(code).expect_value_eq("foobar")
}

// A plain read inside the closure still gets the use site span.
#[test]
fn captured_variable_read_gets_use_site_span() -> Result {
    let code =
        "let x = 'abc'; do { let y = $x; let s = (metadata $y).span; view span $s.start $s.end }";
    test().run(code).expect_value_eq("$x")
}

#[test]
fn row_condition_capture_keeps_definition_span() -> Result {
    let code = r#"let x = 'abc'; [1] | where { let s = (metadata $x).span; (view span $s.start $s.end) == "'abc'" } | length"#;
    test().run(code).expect_value_eq(1)
}
