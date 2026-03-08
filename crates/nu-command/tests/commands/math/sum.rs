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

    let code = format!(
        r#"
            {sample}
            | get meals
            | get calories
            | math sum
        "#
    );

    let outcome: i64 = test().run(&code)?;
    assert_eq!(outcome, 448);
    Ok(())
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
    let mut tester = test().cwd("tests/fixtures/formats");
    for (column_name, expected_value) in answers_for_columns {
        let code = format!(
            "open sample-ps-output.json | select {column_name} | math sum | get {column_name}"
        );
        let result: f64 = tester.run(&code)?;
        assert_eq!(result, expected_value);
    }
    Ok(())
}

#[test]
#[allow(clippy::unreadable_literal)]
#[allow(clippy::float_cmp)]
fn compute_sum_of_table() -> Result {
    let answers_for_columns = [
        ("cpu", 88.257434),
        ("mem", 3032375296.),
        ("virtual", 102579965952.),
    ];
    let mut tester = test().cwd("tests/fixtures/formats");
    for (column_name, expected_value) in answers_for_columns {
        let code = format!(
            "open sample-ps-output.json | select cpu mem virtual | math sum | get {column_name}"
        );
        let result: f64 = tester.run(&code)?;
        assert_eq!(result, expected_value);
    }
    Ok(())
}

#[test]
fn sum_of_a_row_containing_a_table_is_an_error() -> Result {
    let outcome = test()
        .cwd("tests/fixtures/formats")
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
    let outcome: i64 = test().run("const SUM = [1 3] | math sum; $SUM")?;
    assert_eq!(outcome, 4);
    Ok(())
}

#[test]
fn cannot_sum_infinite_range() -> Result {
    let outcome = test().run("0.. | math sum").expect_shell_error()?;

    assert!(matches!(outcome, ShellError::IncorrectValue { .. }));
    Ok(())
}
