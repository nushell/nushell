use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn splits() {
    let sample = r#"
                [[first_name, last_name, rusty_at, type];
                 [Andrés, Robalino, "10/11/2013", A],
                 [JT, Turner, "10/12/2013", B],
                 [Yehuda, Katz, "10/11/2013", A]]
            "#;

    let actual = nu!(pipeline(&format!(
        r#"
                  {sample}
                | group-by rusty_at
                | split-by type
                | get A."10/11/2013"
                | length
            "#
    )));

    assert_eq!(actual.out, "2");
}

#[test]
fn errors_if_no_input() {
    Playground::setup("split_by_no_input", |dirs, _sandbox| {
        let actual = nu!(cwd: dirs.test(), pipeline("split-by type"));

        assert!(actual.err.contains("no input value was piped in"));
    })
}

#[test]
fn errors_if_non_record_input() {
    Playground::setup("split_by_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
        ]);

        let input_mismatch = nu!(cwd: dirs.test(), pipeline("5 | split-by type"));

        assert!(input_mismatch.err.contains("doesn't support int input"));

        let only_supports = nu!(
            cwd: dirs.test(), pipeline(
            "
                ls
                | get name
                | split-by type
            "
        ));

        assert!(only_supports
            .err
            .contains("only Record input data is supported"));
    })
}
