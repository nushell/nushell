use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn wrap_rows_into_a_row() {
    Playground::setup("embed_test_1", |dirs, sandbox| {
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
                | from-csv
                | wrap caballeros
                | get caballeros
                | nth 0
                | get last_name
                | echo $it
            "#
        ));

        assert_eq!(actual, "Robalino");
    })
}

#[test]
fn wrap_rows_into_a_table() {
    Playground::setup("embed_test_2", |dirs, sandbox| {
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
                | from-csv
                | get last_name
                | wrap caballero
                | nth 2
                | get caballero
                | echo $it
            "#
        ));

        assert_eq!(actual, "Katz");
    })
}
