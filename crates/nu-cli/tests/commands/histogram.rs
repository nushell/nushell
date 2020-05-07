use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn summarizes() {
    Playground::setup("histogram_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "los_tres_caballeros.csv",
            r#"
                first_name,last_name,rusty_at
                Andrés,Robalino,Ecuador
                Jonathan,Turner,Estados Unidos
                Yehuda,Katz,Estados Unidos
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open los_tres_caballeros.csv
                | histogram rusty_at countries
                | where rusty_at == "Ecuador"
                | get countries
                | echo $it
            "#
        ));

        assert_eq!(
            actual.out,
            "**************************************************"
        );
        // 50%
    })
}

#[test]
fn help() {
    Playground::setup("histogram_test_help", |dirs, _sandbox| {
        let help_command = nu!(
        cwd: dirs.test(), pipeline(
        r#"
                help histogram
            "#
        ));

        let help_short = nu!(
        cwd: dirs.test(), pipeline(
        r#"
                histogram -h
            "#
        ));

        let help_long = nu!(
        cwd: dirs.test(), pipeline(
        r#"
                histogram --help
            "#
        ));

        assert_eq!(help_short.out, help_command.out);
        assert_eq!(help_long.out, help_command.out);
    })
}
