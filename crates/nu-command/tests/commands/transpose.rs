use nu_protocol::ShellError;
use nu_test_support::prelude::*;
use pretty_assertions::assert_matches;

#[test]
fn row() -> Result {
    let actual: Vec<String> =
        test().run("[[key value]; [foo 1] [foo 2]] | transpose -r | debug")?;
    assert!(actual.iter().any(|row| row.contains("foo: 1")));
    Ok(())
}

#[test]
fn row_but_last() -> Result {
    let actual: Vec<String> =
        test().run("[[key value]; [foo 1] [foo 2]] | transpose -r -l | debug")?;
    assert!(actual.iter().any(|row| row.contains("foo: 2")));
    Ok(())
}

#[test]
fn row_but_all() -> Result {
    let actual: Vec<String> =
        test().run("[[key value]; [foo 1] [foo 2]] | transpose -r -a | debug")?;
    assert!(actual.iter().any(|row| row.contains("foo: [1, 2]")));
    Ok(())
}

#[test]
fn throw_inner_error() -> Result {
    let error_msg = "This message should show up";
    let error = format!("(error make {{ msg: \"{error_msg}\" }})");
    let err = test()
        .run(format!(
            "[[key value]; [foo 1] [foo 2] [{} 3]] | transpose",
            error
        ))
        .expect_shell_error()?;

    assert_contains(error_msg, err.to_string());
    Ok(())
}

#[test]
fn rejects_non_table_stream_input() -> Result {
    let err = test()
        .run("[1 2 3] | each { |it| ($it * 2) } | transpose | to nuon")
        .expect_shell_error()?;

    assert_matches!(
        err,
        ShellError::OnlySupportsThisInputType { exp_input_type, .. }
            if exp_input_type == "table or record"
    );
    Ok(())
}

#[test]
fn missing_columns_become_null() -> Result {
    test()
        .run("[{a: 1} {b: 2} {a: 3, c: 4}] | transpose | to nuon")
        .expect_value_eq(
            r#"[["column0", "column1", "column2", "column3"]; [a, 1, null, 3], [b, null, 2, null], [c, null, null, 4]]"#,
        )
}

#[test]
fn header_row_keeps_first_of_repeated_keys() -> Result {
    test()
        .run("[[key value other]; [foo 1 x] [bar 2 y] [foo 3 z]] | transpose -r | to nuon")
        .expect_value_eq("[[foo, bar]; [1, 2], [x, y]]")
}

#[test]
fn header_row_keep_last_with_interleaved_keys() -> Result {
    test()
        .run("[[key value]; [foo 1] [bar 2] [foo 3]] | transpose -r -l -d | to nuon")
        .expect_value_eq("{foo: 3, bar: 2}")
}

#[test]
fn header_row_keep_all_with_interleaved_keys() -> Result {
    test()
        .run("[[key value]; [foo 1] [bar 2] [foo 3] [foo 4]] | transpose -r -a -d | to nuon")
        .expect_value_eq("{foo: [1, 3, 4], bar: 2}")
}

#[test]
fn repeated_column_names_keep_title() -> Result {
    test()
        .run("[[a b]; [1 2] [3 4]] | transpose x x x | to nuon")
        .expect_value_eq("[[x]; [a], [b]]")
}

#[test]
fn as_record_on_empty_list_returns_empty_record() -> Result {
    // With --as-record the user asked for a record, so an empty table
    // should come back as an empty record rather than an empty list.
    test()
        .run("[] | transpose --as-record | to nuon")
        .expect_value_eq("{}")
}

#[test]
fn as_record_round_trips_empty_record() -> Result {
    // --as-record is the inverse of transpose for records, so the
    // round trip of an empty record has to stay an empty record.
    test()
        .run("{} | transpose | transpose --as-record | to nuon")
        .expect_value_eq("{}")
}

#[test]
fn as_record_on_single_row_unchanged() -> Result {
    test()
        .run("[{a: 1}] | transpose --as-record | to nuon")
        .expect_value_eq(r#"{"column0": a, "column1": 1}"#)
}

#[test]
fn as_record_on_multiple_rows_stays_table() -> Result {
    test()
        .run("[{a: 1, b: 2}] | transpose --as-record | to nuon")
        .expect_value_eq(r#"[["column0", "column1"]; [a, 1], [b, 2]]"#)
}
