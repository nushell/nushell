use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::nu;
use nu_test_support::{pipeline, playground::Playground};

#[test]
fn folds_all_rows_from_internal_and_pipes_it_as_one_stdin_of_external() {
    Playground::setup("internal_to_external_pipe_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "nu_times.csv",
            r#"
                name,rusty_luck,origin
                Jason,1,Canada
                Jonathan,1,New Zealand
                Andrés,1,Ecuador
                AndKitKatz,1,Estados Unidos
            "#,
        )]);

        let actual = nu!(
        cwd: dirs.test(), pipeline(
        r#"
            open nu_times.csv
            | get name
            | chop
            | cococo $it
            "#
        ));

        assert_eq!(actual, "JasonJonathanAndrésAndKitKat");
    })
}
#[test]
fn can_process_one_row_from_internal_and_pipes_it_to_stdin_of_external() {
    let actual = nu!(
        cwd: ".",
        r#"echo "nushelll" | chop"#
    );

    assert_eq!(actual, "nushell");
}
