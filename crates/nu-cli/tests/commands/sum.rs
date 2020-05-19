use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn all() {
    Playground::setup("sum_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "meals.json",
            r#"
                {
                    meals: [
                        {description: "1 large egg", calories: 90},
                        {description: "1 cup white rice", calories: 250},
                        {description: "1 tablespoon fish oil", calories: 108}
                    ]
                }
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open meals.json
                | get meals
                | get calories
                | sum
                | echo $it
            "#
        ));

        assert_eq!(actual.out, "448");
    })
}

#[test]
fn outputs_zero_with_no_input() {
    Playground::setup("sum_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "meals.json",
            r#"
                {
                    meals: [
                        {description: "1 large egg", calories: 90},
                        {description: "1 cup white rice", calories: 250},
                        {description: "1 tablespoon fish oil", calories: 108}
                    ]
                }
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                sum
                | echo $it
            "#
        ));

        assert_eq!(actual.out, "0");
    })
}

#[test]
fn compute_sum_of_individual_row() {
    let answers_for_columns = [("cpu", "104.250050000000008"), ("mem", "8736780288"), ("virtual", "204300193792")];
    for (column_name, expected_value) in answers_for_columns.iter() {
        let actual = nu!(
            cwd: "tests/fixtures/formats/",
            format!("open sample-ps-output.json | select {} | sum | get {}", column_name, column_name)
        );
        assert_eq!(actual.out, *expected_value);
    }
}

#[test]
fn compute_sum_of_table() {
    let answers_for_columns = [("cpu", "104.250050000000008"), ("mem", "8736780288"), ("virtual", "204300193792")];
    for (column_name, expected_value) in answers_for_columns.iter() {
        let actual = nu!(
            cwd: "tests/fixtures/formats/",
            format!("open sample-ps-output.json | select cpu mem virtual | sum | get {}", column_name)
        );
        assert_eq!(actual.out, *expected_value);
    }
}

#[test]
fn sum_of_a_row_containing_a_table_is_an_error() {
    let actual = nu!(
        cwd: "tests/fixtures/formats/",
        "open sample-sys-output.json | sum"
    );
    assert!(actual.err.contains("Attempted to compute the sum of a value that cannot be summed."));
}
