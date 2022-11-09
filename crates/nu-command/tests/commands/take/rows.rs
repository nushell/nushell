use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn rows() {
    Playground::setup("take_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "caballeros.csv",
            r#"
                name,lucky_code
                Andrés,1
                Jonathan,1
                Jason,2
                Yehuda,1
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open caballeros.csv
                | take 3
                | get lucky_code
                | math sum
                "#
        ));

        assert_eq!(actual.out, "4");
    })
}

#[test]
fn rows_with_no_arguments_should_lead_to_error() {
    Playground::setup("take_test_2", |dirs, _sandbox| {
        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"[1 2 3] | take"#
        ));

        assert!(actual.err.contains("missing_positional"));
    })
}
