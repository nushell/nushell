use nu_test_support::fs::Stub::FileWithContent;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn string() {
    Playground::setup("trim_test_1", |dirs, _sandbox| {
        let test_strings = ["\n", " \n ", "\thi\n\n", "\u{2003}a"];
        assert!(test_strings[3].chars().count() == 2);

        for test_string in &test_strings {
            let commandline = format!(
                r#"
                        echo {}
                        | trim
                    "#,
                test_string
            );
            let actual = nu!(
                cwd: dirs.test(), pipeline(&commandline
            ));
            assert_eq!(actual.out, test_string.trim())
        }
    })
}

#[test]
fn row() {
    Playground::setup("trim_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![
            FileWithContent("lines.csv", "lines\n l0\n\tl1\n l2\t \n\n"),
            FileWithContent("lines_trimmed.csv", "lines\nl0\nl1\nl2\n"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open lines.csv
                | trim
            "#
        ));

        let expected = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open lines_trimmed.csv
            "#
        ));

        assert_eq!(actual.out, expected.out)
    })
}

#[test]
fn nested() {
    Playground::setup("trim_test_3", |dirs, sandbox| {
        sandbox.with_files(vec![
            FileWithContent(
                "nested.json",
                r#"{ "l0" : {"l1": {"l2" : {"a" : "  s0", "b" : "\t\ts1\n"} } } }"#,
            ),
            FileWithContent(
                "nested_trimmed.json",
                r#"{ "l0" : {"l1": {"l2" : {"a" : "s0", "b" : "s1"} } } }"#,
            ),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open nested.json
                | trim
            "#
        ));

        let expected = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open nested_trimmed.json
            "#
        ));

        assert_eq!(actual.out, expected.out)
    })
}
