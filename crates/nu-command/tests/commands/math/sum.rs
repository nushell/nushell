use nu_test_support::nu;
use std::str::FromStr;

#[test]
fn all() {
    let sample = r#"{
        meals: [
            {description: "1 large egg", calories: 90},
            {description: "1 cup white rice", calories: 250},
            {description: "1 tablespoon fish oil", calories: 108}
        ]
    }"#;

    let actual = nu!(format!(
        r#"
            {sample}
            | get meals
            | get calories
            | math sum
        "#
    ));

    assert_eq!(actual.out, "448");
}

#[test]
#[allow(clippy::unreadable_literal)]
#[allow(clippy::float_cmp)]
fn compute_sum_of_individual_row() -> Result<(), String> {
    let answers_for_columns = [
        ("cpu", 88.257434),
        ("mem", 3032375296.),
        ("virtual", 102579965952.),
    ];
    for (column_name, expected_value) in answers_for_columns {
        let actual = nu!(
            cwd: "tests/fixtures/formats/",
            format!("open sample-ps-output.json | select {column_name} | math sum | get {column_name}")
        );
        let result =
            f64::from_str(&actual.out).map_err(|_| String::from("Failed to parse float."))?;
        assert_eq!(result, expected_value);
    }
    Ok(())
}

#[test]
#[allow(clippy::unreadable_literal)]
#[allow(clippy::float_cmp)]
fn compute_sum_of_table() -> Result<(), String> {
    let answers_for_columns = [
        ("cpu", 88.257434),
        ("mem", 3032375296.),
        ("virtual", 102579965952.),
    ];
    for (column_name, expected_value) in answers_for_columns {
        let actual = nu!(
            cwd: "tests/fixtures/formats/",
            format!("open sample-ps-output.json | select cpu mem virtual | math sum | get {column_name}")
        );
        let result =
            f64::from_str(&actual.out).map_err(|_| String::from("Failed to parse float."))?;
        assert_eq!(result, expected_value);
    }
    Ok(())
}

#[test]
fn sum_of_a_row_containing_a_table_is_an_error() {
    let actual = nu!(
        cwd: "tests/fixtures/formats/",
        "open sample-sys-output.json | math sum"
    );
    assert!(actual.err.contains("can't convert record"));
}

#[test]
fn const_sum() {
    let actual = nu!("const SUM = [1 3] | math sum; $SUM");
    assert_eq!(actual.out, "4");
}

#[test]
fn cannot_sum_infinite_range() {
    let actual = nu!("0.. | math sum");

    assert!(actual.err.contains("nu::shell::incorrect_value"));
}
