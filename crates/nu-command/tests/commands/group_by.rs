use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn groups() {
    Playground::setup("group_by_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "los_tres_caballeros.csv",
            r#"
                first_name,last_name,rusty_at,type
                Andrés,Robalino,10/11/2013,A
                JT,Turner,10/12/2013,B
                Yehuda,Katz,10/11/2013,A
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open los_tres_caballeros.csv
                | group-by rusty_at
                | get "10/11/2013"
                | length
            "#
        ));

        assert_eq!(actual.out, "2");
    })
}

#[test]
fn errors_if_column_not_found() {
    Playground::setup("group_by_test_3", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "los_tres_caballeros.csv",
            r#"
                first_name,last_name,rusty_at,type
                Andrés,Robalino,10/11/2013,A
                JT,Turner,10/12/2013,B
                Yehuda,Katz,10/11/2013,A
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open los_tres_caballeros.csv
                | group-by ttype
            "#
        ));

        assert!(actual.err.contains("did you mean 'type'"),);
    })
}

#[test]
fn errors_if_input_empty() {
    let actual = nu!("group-by date");
    assert!(actual.err.contains("expected table from pipeline"));
}

#[test]
fn optional_cell_path_works() {
    let actual = nu!("[{foo: 123}, {foo: 234}, {bar: 345}] | group-by foo? | to nuon");
    let expected = r#"{"123": [[foo]; [123]], "234": [[foo]; [234]]}"#;
    assert_eq!(actual.out, expected)
}
