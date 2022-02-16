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
                Jonathan,Turner,10/12/2013,B
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
fn errors_if_given_unknown_column_name() {
    Playground::setup("group_by_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "los_tres_caballeros.json",
            r#"
                {
                    "nu": {
                        "committers": [
                            {"name": "Andrés N. Robalino"},
                            {"name": "Jonathan Turner"},
                            {"name": "Yehuda Katz"}
                        ],
                        "releases": [
                            {"version": "0.2"}
                            {"version": "0.8"},
                            {"version": "0.9999999"}
                        ],
                        "0xATYKARNU": [
                            ["Th", "e", " "],
                            ["BIG", " ", "UnO"],
                            ["punto", "cero"]
                        ]
                    }
                }
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open los_tres_caballeros.json
                | group-by { get nu.releases.version }
            "#
        ));

        assert!(actual
            .err
            .contains("requires a table with one value for grouping"));
    })
}

#[test]
fn errors_if_block_given_evaluates_more_than_one_row() {
    Playground::setup("group_by_test_3", |dirs, sandbox| {
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
                | group-by ttype
            "#
        ));

        assert!(actual.err.contains("value originates here"),);
        assert!(actual.err.contains("cannot find column"),);
    })
}

#[test]
fn errors_if_input_empty() {
    Playground::setup("group_by_empty_test", |dirs, _sandbox| {
        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
            group-by date
        "#
        ));

        assert!(actual.err.contains("expected table from pipeline"));
    });
}
