use nu_protocol::test_record;
use nu_test_support::{fs::fixtures, prelude::*};

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
    let mut tester = test().cwd(fixtures().join("formats"));
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
        .cwd(fixtures().join("formats"))
        .run(code)
        .expect_value_eq(expected)
}

#[test]
fn sum_of_a_row_containing_a_table_is_an_error() -> Result {
    let outcome = test()
        .cwd(fixtures().join("formats"))
        .run("open sample-sys-output.json | math sum")
        .expect_shell_error()?;
    match outcome {
        ShellError::CantConvert { from_type, .. } => {
            assert_contains("record", from_type);
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
