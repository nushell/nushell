use nu_protocol::test_record;
use nu_test_support::prelude::*;

#[test]
fn all() -> Result {
    let sample = r#"{
        meals: [
            {description: "1 large egg", calories: 90},
            {description: "1 cup white rice", calories: 250},
            {description: "1 tablespoon fish oil", calories: 108}
        ]
    }"#;

    let code = "
        from nuon
        | get meals
        | get calories
        | math sum
    ";

    test().run_with_data(code, sample).expect_value_eq(448)
}

#[test]
#[allow(clippy::unreadable_literal)]
#[allow(clippy::float_cmp)]
fn compute_sum_of_individual_row() -> Result {
    let answers_for_columns = [
        ("cpu", 88.257434),
        ("mem", 3032375296.),
        ("virtual", 102579965952.),
    ];
    let mut tester = test().cwd(FIXTURES.join("formats"));
    for (column_name, expected_value) in answers_for_columns {
        let () = tester.run_with_data("let column = into cell-path", [column_name])?;
        let code = "
            open sample-ps-output.json
            | select $column
            | math sum
            | get $column
        ";
        tester.run(code).expect_value_eq(expected_value)?;
    }
    Ok(())
}

#[test]
#[allow(clippy::unreadable_literal)]
#[allow(clippy::float_cmp)]
fn compute_sum_of_table() -> Result {
    let code = "
        open sample-ps-output.json
        | select cpu mem virtual
        | math sum
    ";

    let expected = test_record! {
        "cpu" => 88.257434,
        "mem" => 3032375296.,
        "virtual" => 102579965952.,
    };

    test()
        .cwd(FIXTURES.join("formats"))
        .run(code)
        .expect_value_eq(expected)
}

#[test]
fn sum_of_a_row_containing_a_table_is_an_error() -> Result {
    let outcome = test()
        .cwd(FIXTURES.join("formats"))
        .run("open sample-sys-output.json | math sum")
        .expect_shell_error()?;
    match outcome {
        ShellError::OnlySupportsThisInputType { wrong_type, .. } => {
            assert_contains("record", wrong_type);
        }
        err => return Err(err.into()),
    }
    Ok(())
}

#[test]
fn const_sum() -> Result {
    test()
        .run("const SUM = [1 3] | math sum; $SUM")
        .expect_value_eq(4)
}

#[test]
fn cannot_sum_infinite_range() -> Result {
    let outcome = test().run("0.. | math sum").expect_shell_error()?;

    assert!(matches!(outcome, ShellError::IncorrectValue { .. }));
    Ok(())
}

#[test]
fn sum_list_valued_record_columns() -> Result {
    test()
        .run("{alice: [1 2 3], bob: [4 5]} | math sum")
        .expect_value_eq(test_record! {
            "alice" => 6,
            "bob" => 9,
        })
}

#[test]
fn sum_record_cell_path_only_touches_named_column() -> Result {
    test()
        .run("{alice: [1 2 3], bob: [4 5 6]} | math sum alice")
        .expect_value_eq(test_value!({
            alice: 6,
            bob: [4, 5, 6],
        }))
}

#[test]
fn sum_record_scalar_cell_path_keeps_value() -> Result {
    test()
        .run("{a: 5, b: [1 2]} | math sum a")
        .expect_value_eq(test_value!({
            a: 5,
            b: [1, 2],
        }))
}

#[test]
fn sum_table_cell_path_sums_the_column() -> Result {
    // A table column is summed over all rows, not row by row (where each
    // scalar cell would just "sum" to itself and the table come back as is).
    test()
        .run("[[a]; [1] [2]] | math sum a")
        .expect_value_eq(test_record! { "a" => 3 })
}

#[test]
fn sum_table_filesize_column() -> Result {
    let expected: Value = test().run("{size: 3kb}")?;
    test()
        .run("[[name size]; [a.md 1kb] [b.md 2kb]] | math sum size")
        .expect_value_eq(expected)
}

#[test]
fn sum_table_cell_paths_keep_only_named_columns_in_order() -> Result {
    test()
        .run("[[name a b c]; [x 1 2 3] [y 4 5 6]] | math sum c a | to nuon")
        .expect_value_eq("{c: 9, a: 5}")
}

#[test]
fn sum_table_repeated_column_is_summed_once() -> Result {
    // `select a a` keeps one `a` column, so the sum must not double.
    test()
        .run("[[a]; [1] [2]] | math sum a a")
        .expect_value_eq(test_record! { "a" => 3 })
}

#[test]
fn sum_table_cell_paths_match_select_then_sum() -> Result {
    let code = "
        let t = [[name a b]; [x 1 2.5] [y 3 4.5]]
        ($t | math sum a b) == ($t | select a b | math sum)
    ";
    test().run(code).expect_value_eq(true)
}

#[test]
fn sum_table_nested_cell_path_uses_select_column_name() -> Result {
    test()
        .run("[[x]; [{y: 1}] [{y: 2}]] | math sum x.y")
        .expect_value_eq(test_record! { "x.y" => 3 })
}

#[test]
fn sum_table_stream_cell_path() -> Result {
    test()
        .run("1..3 | each {|x| {a: $x} } | math sum a")
        .expect_value_eq(test_record! { "a" => 6 })
}

#[test]
fn sum_table_cell_path_in_const() -> Result {
    test()
        .run("const S = [[a]; [1] [2]] | math sum a; $S")
        .expect_value_eq(test_record! { "a" => 3 })
}

#[test]
fn sum_table_missing_column_is_error() -> Result {
    let err = test().run("[[a]; [1]] | math sum b").expect_shell_error()?;
    assert!(matches!(err, ShellError::CantFindColumn { .. }));
    Ok(())
}

#[test]
fn sum_table_list_cells_are_an_error_like_select_then_sum() -> Result {
    let err = test()
        .run("[[a]; [[1 2]] [[3 4]]] | math sum a")
        .expect_shell_error()?;
    assert!(matches!(err, ShellError::OnlySupportsThisInputType { .. }));
    Ok(())
}

#[test]
fn sum_table_error_row_is_reported() -> Result {
    let code = r#"
        [1 2] | each {|x| if $x == 2 { error make {msg: "boom"} } else { {a: $x} } }
        | math sum a
    "#;
    let err = test().run(code).expect_shell_error()?;
    assert_contains("boom", format!("{err:?}"));
    Ok(())
}

#[test]
fn sum_stream_first_item_error_is_reported() -> Result {
    // `describe` would run, and name a stream of errors, if the error were
    // passed along as a row instead of returned.
    let code = r#"
        [1 2] | each {|x| if $x == 1 { error make {msg: "boom"} } else { {a: $x} } }
        | math sum a
        | describe
    "#;
    let err = test().run(code).expect_shell_error()?;
    assert_contains("boom", format!("{err:?}"));
    Ok(())
}

#[test]
fn sum_endless_stream_of_lists_keeps_streaming() -> Result {
    // Only a table is collected; a stream of lists is still reduced per item.
    // `do` hides `math sum`'s declared output type, which `first` would reject.
    test()
        .run("do {|| 1.. | each {|x| [$x $x]} | math sum 0 } | first 2 | to nuon")
        .expect_value_eq("[[1, 1], [2, 2]]")
}

#[test]
fn sum_empty_list_cell_path_returns_input() -> Result {
    test()
        .run("[] | math sum a | to nuon")
        .expect_value_eq("[]")
}

#[test]
fn sum_unsupported_record_field_is_top_level_error() -> Result {
    let err = test()
        .run("{alice: true, bob: [1 2]} | math sum")
        .expect_shell_error()?;
    assert!(matches!(err, ShellError::OnlySupportsThisInputType { .. }));
    Ok(())
}

#[test]
fn overflow_error_is_consistent_between_list_and_table() -> Result {
    // Large durations that sum beyond i64::MAX nanoseconds
    let durations = "[618019200000000000ns, 650422800000000000ns, 652579200000000000ns, 657849600000000000ns, 660873600000000000ns, 662342400000000000ns, 664416000000000000ns, 667782000000000000ns, 669855600000000000ns, 673311600000000000ns, 675903600000000000ns, 677462400000000000ns, 681609600000000000ns, 683766000000000000ns]";

    let list_err = test()
        .run(format!("{durations} | math sum"))
        .expect_shell_error()?;

    let table_err = test()
        .run(format!("{durations} | wrap d | math sum"))
        .expect_shell_error()?;

    assert!(
        std::mem::discriminant(&list_err) == std::mem::discriminant(&table_err),
        "list and table paths produced different error variants:\n  list:  {list_err:?}\n  table: {table_err:?}"
    );
    Ok(())
}
