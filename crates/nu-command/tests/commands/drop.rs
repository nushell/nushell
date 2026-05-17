use nu_test_support::prelude::*;

#[test]
fn columns() -> Result {
    let code = "
        echo [
            [arepas, color];
            [3,  white]
            [8, yellow]
            [4,  white]
        ] | drop column | columns | length
    ";

    test().run(code).expect_value_eq(1)
}

#[test]
fn drop_columns_positive_value() -> Result {
    let err = test()
        .run("echo [[a, b];[1,2]] | drop column -1")
        .expect_shell_error()?;
    assert!(matches!(err, ShellError::NeedsPositiveValue { .. }));
    Ok(())
}

#[test]
fn more_columns_than_table_has() -> Result {
    let code = "
        echo [
            [arepas, color];
            [3,  white]
            [8, yellow]
            [4,  white]
        ] | drop column 3 | columns | is-empty
    ";

    test().run(code).expect_value_eq(true)
}

#[test]
fn rows() -> Result {
    let code = "
        echo [
            [arepas];
            [3]
            [8]
            [4]
        ]
        | drop 2
        | get arepas
        | math sum
    ";

    test().run(code).expect_value_eq(3)
}

#[test]
fn more_rows_than_table_has() -> Result {
    test().run("[date] | drop 50 | length").expect_value_eq(0)
}

#[test]
fn nth_range_inclusive() -> Result {
    test()
        .run("echo 10..15 | drop nth (2..3)")
        .expect_value_eq([10, 11, 14, 15])
}

#[test]
fn nth_range_exclusive() -> Result {
    test()
        .run("echo 10..15 | drop nth (1..<3)")
        .expect_value_eq([10, 13, 14, 15])
}

#[test]
fn nth_missing_first_argument() -> Result {
    let err = test()
        .run(r#"echo 10..15 | drop nth """#)
        .expect_shell_error()?;
    match err {
        ShellError::TypeMismatch { err_message, .. } => {
            assert_eq!(err_message, "Expected int or range");
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[test]
fn fail_on_non_iterator() -> Result {
    let err = test().run("1 | drop 50").expect_parse_error()?;
    assert!(matches!(err, ParseError::InputMismatch { .. }));
    Ok(())
}

#[test]
fn disjoint_columns_first_row_becomes_empty() -> Result {
    let code = "
        [{a: 1}, {b: 2, c: 3}]
        | drop column
        | columns
        | to nuon
    ";

    test().run(code).expect_value_eq("[b, c]")
}

#[test]
fn disjoint_columns() -> Result {
    let code = "
        [{a: 1, b: 2}, {c: 3}]
        | drop column
        | columns
        | to nuon
    ";

    test().run(code).expect_value_eq("[a, c]")
}

#[test]
fn record() -> Result {
    test()
        .run("{a: 1, b: 2} | drop column | to nuon")
        .expect_value_eq("{a: 1}")
}

#[test]
fn more_columns_than_record_has() -> Result {
    test()
        .run("{a: 1, b: 2} | drop column 3 | to nuon")
        .expect_value_eq("{}")
}

#[test]
fn drop_single_index() -> Result {
    test()
        .run("echo 10..15 | drop nth 2")
        .expect_value_eq([10, 11, 13, 14, 15])
}

#[test]
fn drop_multiple_indices() -> Result {
    test()
        .run("echo 0..10 | drop nth 1 3")
        .expect_value_eq([0, 2, 4, 5, 6, 7, 8, 9, 10])
}

#[test]
fn drop_inclusive_range() -> Result {
    test()
        .run("echo 10..15 | drop nth (2..4)")
        .expect_value_eq([10, 11, 15])
}

#[test]
fn drop_exclusive_range() -> Result {
    test()
        .run("echo 10..15 | drop nth (2..<4)")
        .expect_value_eq([10, 11, 14, 15])
}

#[test]
fn drop_unbounded_range() -> Result {
    test()
        .run("echo 0..5 | drop nth 3..")
        .expect_value_eq([0, 1, 2])
}

#[test]
fn drop_multiple_ranges_including_unbounded() -> Result {
    let code = "
    0..30
    | drop nth 0..10 20..
    ";

    test()
        .run(code)
        .expect_value_eq([11, 12, 13, 14, 15, 16, 17, 18, 19])
}

#[test]
fn drop_combination_of_unbounded_range_and_single_index() -> Result {
    let code = "
        echo 0..15
        | drop nth 10.. 5
        ";

    test()
        .run(code)
        .expect_value_eq([0, 1, 2, 3, 4, 6, 7, 8, 9])
}

#[test]
fn drop_combination_of_two_unbounded_ranges() -> Result {
    let code = "
        echo 0..150
        | drop nth 0..100 999..
        ";

    let expected: Vec<u32> = (101..=150).collect();
    let actual: Vec<u32> = test().run(code)?;
    assert_eq!(actual, expected);
    Ok(())
}
