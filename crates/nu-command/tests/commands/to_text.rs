use rstest::rstest;
use rstest_reuse::{apply, template};

use nu_test_support::prelude::*;
use nu_utils::consts::LINE_SEPARATOR_STR;

#[template]
#[rstest]
#[case(&[])]
#[case(&["a"])]
#[case(&["a", "b"])]
fn input_template(#[case] input: &[&str]) -> Result {}

#[apply(input_template)]
fn list(#[case] input: &[&str]) -> Result {
    let mut expect = input.join(LINE_SEPARATOR_STR);
    if !expect.is_empty() {
        expect += LINE_SEPARATOR_STR;
    }
    test()
        .run_with_data("to text", input.to_vec())
        .expect_value_eq(expect)
}

// The output should be the same when `to text` gets a ListStream instead of a Value::List.
#[apply(input_template)]
fn list_stream(#[case] input: &[&str]) -> Result {
    let mut expect = input.join(LINE_SEPARATOR_STR);
    if !expect.is_empty() {
        expect += LINE_SEPARATOR_STR;
    }
    test()
        .run_with_data("each {} | to text", input.to_vec())
        .expect_value_eq(expect)
}

#[apply(input_template)]
fn list_no_newline(#[case] input: &[&str]) -> Result {
    let expect = input.join(LINE_SEPARATOR_STR);
    test()
        .run_with_data("to text --no-newline", input.to_vec())
        .expect_value_eq(expect)
}

// The output should be the same when `to text` gets a ListStream instead of a Value::List.
#[apply(input_template)]
fn list_stream_no_newline(#[case] input: &[&str]) -> Result {
    let expect = input.join(LINE_SEPARATOR_STR);
    test()
        .run_with_data("each {} | to text --no-newline", input.to_vec())
        .expect_value_eq(expect)
}
