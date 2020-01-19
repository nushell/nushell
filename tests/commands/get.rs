use nu_test_support::fs::Stub::FileWithContent;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, nu_error, pipeline};

#[test]
fn fetches_a_row() {
    Playground::setup("get_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                nu_party_venue = "zion"
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open sample.toml
                | get nu_party_venue
                | echo $it
            "#
        ));

        assert_eq!(actual, "zion");
    })
}

#[test]
fn fetches_by_index() {
    Playground::setup("get_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                [package]
                name = "nu"
                version = "0.4.1"
                authors = ["Yehuda Katz <wycats@gmail.com>", "Jonathan Turner <jonathan.d.turner@gmail.com>", "Andrés N. Robalino <andres@androbtech.com>"]
                description = "When arepas shells are tasty and fun."
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open sample.toml
                | get package.authors.2
                | echo $it
            "#
        ));

        assert_eq!(actual, "Andrés N. Robalino <andres@androbtech.com>");
    })
}
#[test]
fn fetches_by_column_path() {
    Playground::setup("get_test_3", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                [package]
                name = "nu"
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open sample.toml
                | get package.name
                | echo $it
            "#
        ));

        assert_eq!(actual, "nu");
    })
}

#[test]
fn column_paths_are_either_double_quoted_or_regular_unquoted_words_separated_by_dot() {
    Playground::setup("get_test_4", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                [package]
                9999 = ["Yehuda Katz <wycats@gmail.com>", "Jonathan Turner <jonathan.d.turner@gmail.com>", "Andrés N. Robalino <andres@androbtech.com>"]
                description = "When arepas shells are tasty and fun."
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open sample.toml
                | get package."9999"
                | count
                | echo $it
            "#
        ));

        assert_eq!(actual, "3");
    })
}

#[test]
fn fetches_more_than_one_column_path() {
    Playground::setup("get_test_5", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                [[fortune_tellers]]
                name = "Andrés N. Robalino"
                arepas = 1

                [[fortune_tellers]]
                name = "Jonathan Turner"
                arepas = 1

                [[fortune_tellers]]
                name = "Yehuda Katz"
                arepas = 1
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open sample.toml
                | get fortune_tellers.2.name fortune_tellers.0.name fortune_tellers.1.name
                | nth 2
                | echo $it
            "#
        ));

        assert_eq!(actual, "Jonathan Turner");
    })
}

#[test]
fn errors_fetching_by_column_not_present() {
    Playground::setup("get_test_6", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                [taconushell]
                sentence_words = ["Yo", "quiero", "taconushell"]
            "#,
        )]);

        let actual = nu_error!(
            cwd: dirs.test(), pipeline(
            r#"
                open sample.toml
                | get taco
            "#
        ));

        assert!(
            actual.contains("Unknown column"),
            format!("actual: {:?}", actual)
        );
        assert!(
            actual.contains("There isn't a column named 'taco'"),
            format!("actual: {:?}", actual)
        );
        assert!(
            actual.contains("Perhaps you meant 'taconushell'?"),
            format!("actual: {:?}", actual)
        )
    })
}

#[test]
fn errors_fetching_by_column_using_a_number() {
    Playground::setup("get_test_7", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                [spanish_lesson]
                0 = "can only be fetched with 0 double quoted."
            "#,
        )]);

        let actual = nu_error!(
            cwd: dirs.test(), pipeline(
            r#"
                open sample.toml
                | get spanish_lesson.0
            "#
        ));

        assert!(
            actual.contains("No rows available"),
            format!("actual: {:?}", actual)
        );
        assert!(
            actual.contains("A row at '0' can't be indexed."),
            format!("actual: {:?}", actual)
        );
        assert!(
            actual.contains("Appears to contain columns. Columns available: 0"),
            format!("actual: {:?}", actual)
        )
    })
}
#[test]
fn errors_fetching_by_index_out_of_bounds() {
    Playground::setup("get_test_8", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                [spanish_lesson]
                sentence_words = ["Yo", "quiero", "taconushell"]
            "#,
        )]);

        let actual = nu_error!(
            cwd: dirs.test(), pipeline(
            r#"
                open sample.toml
                | get spanish_lesson.sentence_words.3
            "#
        ));

        assert!(
            actual.contains("Row not found"),
            format!("actual: {:?}", actual)
        );
        assert!(
            actual.contains("There isn't a row indexed at 3"),
            format!("actual: {:?}", actual)
        );
        assert!(
            actual.contains("The table only has 3 rows (0 to 2)"),
            format!("actual: {:?}", actual)
        )
    })
}
