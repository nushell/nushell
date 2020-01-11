use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn regular_columns() {
    Playground::setup("pick_test_1", |dirs, sandbox| {
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

#[test]
fn complex_nested_columns() {
    Playground::setup("pick_test_2", |dirs, sandbox| {
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
                | pick nu.0xATYKARNU nu.committers.name nu.releases.version
                | where $it."nu.releases.version" > "0.8"
                | get "nu.releases.version"
                | echo $it
            "#
        ));

        assert_eq!(actual, "0.9999999");
    })
}

#[test]
fn allows_if_given_unknown_column_name_is_missing() {
    Playground::setup("pick_test_3", |dirs, sandbox| {
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
                | pick rrusty_at first_name
                | count
                | echo $it
            "#
        ));

        assert_eq!(actual, "3");
    })
}
