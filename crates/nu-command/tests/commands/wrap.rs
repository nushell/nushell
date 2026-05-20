use nu_test_support::prelude::*;

const SAMPLE: &str = "[
    [first_name, last_name];
    [Andrés, Robalino],
    [JT, Turner],
    [Yehuda, Katz]
]";

#[test]
fn wrap_rows_into_a_row() -> Result {
    let code = "
        from nuon
        | wrap caballeros
        | get caballeros
        | get 0
        | get last_name
    ";

    test()
        .run_with_data(code, SAMPLE)
        .expect_value_eq("Robalino")
}

#[test]
fn wrap_rows_into_a_table() -> Result {
    let code = "
        from nuon
        | get last_name
        | wrap caballero
        | get 2
        | get caballero
    ";

    test().run_with_data(code, SAMPLE).expect_value_eq("Katz")
}
