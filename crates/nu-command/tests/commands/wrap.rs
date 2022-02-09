use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn wrap_rows_into_a_row() {
    Playground::setup("wrap_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "los_tres_caballeros.txt",
            r#"
                first_name,last_name
                Andrés,Robalino
                Jonathan,Turner
                Yehuda,Katz
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open los_tres_caballeros.txt
                | from csv
                | wrap caballeros
                | get caballeros
                | get 0
                | get last_name
            "#
        ));

        assert_eq!(actual.out, "Robalino");
    })
}

#[test]
fn wrap_rows_into_a_table() {
    Playground::setup("wrap_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "los_tres_caballeros.txt",
            r#"
                first_name,last_name
                Andrés,Robalino
                Jonathan,Turner
                Yehuda,Katz
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open los_tres_caballeros.txt
                | from csv
                | get last_name
                | wrap caballero
                | get 2
                | get caballero
            "#
        ));

        assert_eq!(actual.out, "Katz");
    })
}
