use nu_test_support::fs::Stub::{EmptyFile, FileWithContentToBeTrimmed};
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn splits() {
    Playground::setup("split_by_test_1", |dirs, sandbox| {
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
                | group-by rusty_at
                | split-by type
                | get A."10/11/2013"
                | length
            "#
        ));

        assert_eq!(actual.out, "2");
    })
}

#[test]
fn errors_if_no_table_given_as_input() {
    Playground::setup("split_by_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls
                | get name
                | split-by type
            "#
        ));

        assert!(actual.err.contains("requires a table"));
    })
}
