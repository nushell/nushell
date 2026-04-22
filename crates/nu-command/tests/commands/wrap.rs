use nu_test_support::prelude::*;

const SAMPLE: &str = "[
    [first_name, last_name];
    [Andrés, Robalino],
    [JT, Turner],
    [Yehuda, Katz]
]";

#[test]
fn wrap_rows_into_a_row() -> Result {
    let code = format!(
        "
            {SAMPLE}
            | wrap caballeros
            | get caballeros
            | get 0
            | get last_name
        "
    );

    test().run(code).expect_value_eq("Robalino")
}

#[test]
fn wrap_rows_into_a_table() -> Result {
    let code = format!(
        "
            {SAMPLE}
            | get last_name
            | wrap caballero
            | get 2
            | get caballero
        "
    );

    test().run(code).expect_value_eq("Katz")
}
