use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, nu_error, pipeline};

#[test]
fn columns() {
    Playground::setup("pick_by_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "los_tres_caballeros.csv",
            r#"
                first_name,last_name,rusty_at,type
                Andrés,Robalino,10/11/2013,A
                Jonathan,Turner,10/12/2013,B
                Yehuda,Katz,10/11/2013,A
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open los_tres_caballeros.csv
                | pick rusty_at last_name
                | nth 0
                | get last_name
                | echo $it
            "#
        ));

        assert_eq!(actual, "Robalino");
    })
}

#[should_panic]
#[test]
fn errors_if_given_unknown_column_name_is_missing() {
    Playground::setup("pick_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "los_tres_caballeros.csv",
            r#"
                first_name,last_name,rusty_at,type
                Andrés,Robalino,10/11/2013,A
                Jonathan,Turner,10/12/2013,B
                Yehuda,Katz,10/11/2013,A
            "#,
        )]);

        let actual = nu_error!(
            cwd: dirs.test(), pipeline(
            r#"
                open los_tres_caballeros.csv
                | pick rrusty_at
            "#
        ));

        assert!(actual.contains("Unknown column"));
    })
}
