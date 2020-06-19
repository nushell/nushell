use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn adds_value_provided_if_column_is_empty() {
    Playground::setup("is_empty_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "likes.csv",
            r#"
                first_name,last_name,rusty_at,likes
                Andrés,Robalino,10/11/2013,1
                Jonathan,Turner,10/12/2013,1
                Jason,Gedge,10/11/2013,1
                Yehuda,Katz,10/11/2013,
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open likes.csv
                | empty? likes 1
                | get likes
                | math sum
                | echo $it
            "#
        ));

        assert_eq!(actual.out, "4");
    })
}

#[test]
fn adds_value_provided_for_columns_that_are_empty() {
    Playground::setup("is_empty_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "checks.json",
            r#"
                [
                    {"boost": 1, "check": []},
                    {"boost": 1, "check": ""},
                    {"boost": 1, "check": {}},
                    {"boost": null, "check": ["" {} [] ""]}
                ]

            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open checks.json
                | empty? boost check 1
                | get boost check
                | math sum
                | echo $it
            "#
        ));

        assert_eq!(actual.out, "8");
    })
}

#[test]
fn value_emptiness_check() {
    Playground::setup("is_empty_test_3", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "checks.json",
            r#"
                {
                    "are_empty": [
                        {"check": []},
                        {"check": ""},
                        {"check": {}},
                        {"check": ["" {} [] ""]}
                    ]
                }
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open checks.json
                | get are_empty.check
                | empty?
                | where $it
                | count
                | echo $it
            "#
        ));

        assert_eq!(actual.out, "4");
    })
}
