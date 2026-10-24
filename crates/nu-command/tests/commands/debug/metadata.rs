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
