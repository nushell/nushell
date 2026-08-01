use nu_test_support::prelude::*;
use pretty_assertions::assert_matches;
use rstest::rstest;

/// Empty ListStreams used to go through the table helper and return `{}`
/// instead of matching empty-list reducer behavior. These cases lock that in.

#[rstest]
#[case::max("math max")]
#[case::min("math min")]
#[case::sum("math sum")]
#[case::product("math product")]
#[case::avg("math avg")]
#[case::median("math median")]
#[case::stddev("math stddev")]
#[case::variance("math variance")]
fn empty_stream_matches_empty_list_error(#[case] cmd: &str) -> Result {
    let list_err = test()
        .run(format!("[] | {cmd}"))
        .expect_shell_error()?;
    let stream_err = test()
        .run(format!("[] | each {{ $in }} | {cmd}"))
        .expect_shell_error()?;
    let try_stream_err = test()
        .run(format!("['x'] | each {{ try {{ into int }} }} | {cmd}"))
        .expect_shell_error()?;
    let empty_table_stream_err = test()
        .run(format!("[{{a: 1}}] | take 0 | {cmd}"))
        .expect_shell_error()?;

    assert_matches!(list_err, ShellError::UnsupportedInput { .. });
    assert_matches!(stream_err, ShellError::UnsupportedInput { .. });
    assert_matches!(try_stream_err, ShellError::UnsupportedInput { .. });
    assert_matches!(empty_table_stream_err, ShellError::UnsupportedInput { .. });
    Ok(())
}

#[test]
fn empty_stream_mode_matches_empty_list() -> Result {
    test().run("[] | math mode").expect_value_eq(test_value!([]))?;
    test()
        .run("[] | each { $in } | math mode")
        .expect_value_eq(test_value!([]))?;
    test()
        .run("['x'] | each { try { into int } } | math mode")
        .expect_value_eq(test_value!([]))
}

#[test]
fn non_empty_streams_still_reduce() -> Result {
    test()
        .run("[1 5 3] | each { $in } | math max")
        .expect_value_eq(5)?;
    test()
        .run("[{a: 1} {a: 3}] | each { $in } | math max")
        .expect_value_eq(test_record! { "a" => 3 })?;
    test()
        .run("[1 2 3] | each { $in } | math sum")
        .expect_value_eq(6)
}
