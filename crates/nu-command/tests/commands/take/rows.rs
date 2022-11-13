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

#[test]
fn fails_on_string() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
                "foo bar" | take 2
            "#
    ));

    assert!(actual.err.contains("unsupported_input"));
}

#[test]
// covers a situation where `take` used to behave strangely on list<binary> input
fn works_with_binary_list() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        ([0x[01 11]] | take 1 | get 0) == 0x[01 11]
            "#
    ));

    assert_eq!(actual.out, "true");
}
