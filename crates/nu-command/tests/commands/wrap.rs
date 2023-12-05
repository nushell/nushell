use nu_test_support::{nu, pipeline};

#[test]
fn wrap_rows_into_a_row() {
    let sample = r#"
                [[first_name, last_name];
                 [Andrés, Robalino],
                 [JT, Turner],
                 [Yehuda, Katz]]
            "#;

    let actual = nu!(pipeline(&format!(
        "
                {sample}
                | wrap caballeros
                | get caballeros
                | get 0
                | get last_name
            "
    )));

    assert_eq!(actual.out, "Robalino");
}

#[test]
fn wrap_rows_into_a_table() {
    let sample = r#"
                [[first_name, last_name];
                 [Andrés, Robalino],
                 [JT, Turner],
                 [Yehuda, Katz]]
            "#;

    let actual = nu!(pipeline(&format!(
        "
                {sample}
                | get last_name
                | wrap caballero
                | get 2
                | get caballero
            "
    )));

    assert_eq!(actual.out, "Katz");
}
