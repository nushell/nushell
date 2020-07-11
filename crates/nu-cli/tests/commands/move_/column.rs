use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn moves_a_column_before() {
    Playground::setup("move_column_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "sample.csv",
            r#"
                column1,column2,column3,...,column98,column99,column100
                -------,-------,-------,---,--------,   A    ,---------
                -------,-------,-------,---,--------,   N    ,---------
                -------,-------,-------,---,--------,   D    ,---------
                -------,-------,-------,---,--------,   R    ,---------
                -------,-------,-------,---,--------,   E    ,---------
                -------,-------,-------,---,--------,   S    ,---------
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open sample.csv
                | move column column99 --before column1
                | rename chars
                | get chars
                | trim
                | str collect
                | echo $it
            "#
        ));

        assert!(actual.out.contains("ANDRES"));
    })
}

#[test]
fn moves_columns_before() {
    Playground::setup("move_column_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "sample.csv",
            r#"
                column1,column2,column3,...,column98,column99,column100
                -------,-------,   A   ,---,--------,   N    ,---------
                -------,-------,   D   ,---,--------,   R    ,---------
                -------,-------,   E   ,---,--------,   S    ,---------
                -------,-------,   :   ,---,--------,   :    ,---------
                -------,-------,   J   ,---,--------,   O    ,---------
                -------,-------,   N   ,---,--------,   A    ,---------
                -------,-------,   T   ,---,--------,   H    ,---------
                -------,-------,   A   ,---,--------,   N    ,---------
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open sample.csv
                | move column column99 column3 --before column2
                | rename _ chars_1 chars_2
                | get chars_2 chars_1
                | trim
                | str collect
                | echo $it
            "#
        ));

        assert!(actual.out.contains("ANDRES::JONATHAN"));
    })
}

#[test]
fn moves_a_column_after() {
    Playground::setup("move_column_test_3", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "sample.csv",
            r#"
                column1,column2,letters,...,column98,and_more,column100
                -------,-------,   A   ,---,--------,   N    ,---------
                -------,-------,   D   ,---,--------,   R    ,---------
                -------,-------,   E   ,---,--------,   S    ,---------
                -------,-------,   :   ,---,--------,   :    ,---------
                -------,-------,   J   ,---,--------,   O    ,---------
                -------,-------,   N   ,---,--------,   A    ,---------
                -------,-------,   T   ,---,--------,   H    ,---------
                -------,-------,   A   ,---,--------,   N    ,---------
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open sample.csv
                | move column letters --after and_more
                | move column letters and_more --before column2
                | rename _ chars_1 chars_2
                | get chars_1 chars_2
                | trim
                | str collect
                | echo $it
            "#
        ));

        assert!(actual.out.contains("ANDRES::JONATHAN"));
    })
}

#[test]
fn moves_columns_after() {
    Playground::setup("move_column_test_4", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "sample.csv",
            r#"
                column1,column2,letters,...,column98,and_more,column100
                -------,-------,   A   ,---,--------,   N    ,---------
                -------,-------,   D   ,---,--------,   R    ,---------
                -------,-------,   E   ,---,--------,   S    ,---------
                -------,-------,   :   ,---,--------,   :    ,---------
                -------,-------,   J   ,---,--------,   O    ,---------
                -------,-------,   N   ,---,--------,   A    ,---------
                -------,-------,   T   ,---,--------,   H    ,---------
                -------,-------,   A   ,---,--------,   N    ,---------
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open sample.csv
                | move column letters and_more --after column1
                | get
                | nth 1 2
                | str collect
                | echo $it
            "#
        ));

        assert!(actual.out.contains("lettersand_more"));
    })
}
