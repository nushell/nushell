mod helpers;

use helpers as h;
use helpers::{Playground, Stub::*};

#[test]
fn get() {
    Playground::setup("get_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                nu_party_venue = "zion"
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), h::pipeline(
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
fn fetches_by_index_from_a_given_table() {
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
            cwd: dirs.test(), h::pipeline(
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
fn supports_fetching_rows_from_tables_using_columns_named_as_numbers() {
    Playground::setup("get_test_3", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                [package]
                0 = "nu"
                1 = "0.4.1"
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), h::pipeline(
            r#"
                open sample.toml
                | get package.1
                | echo $it
            "#
        ));

        assert_eq!(actual, "0.4.1");
    })
}

#[test]
fn can_fetch_tables_or_rows_using_numbers_in_column_path() {
    Playground::setup("get_test_4", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                [package]
                0 = "nu"
                1 = "0.4.1"
                2 = ["Yehuda Katz <wycats@gmail.com>", "Jonathan Turner <jonathan.d.turner@gmail.com>", "Andrés N. Robalino <andres@androbtech.com>"]
                description = "When arepas shells are tasty and fun."
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), h::pipeline(
            r#"
                open sample.toml
                | get package.2.1
                | echo $it
            "#
        ));

        assert_eq!(actual, "Jonathan Turner <jonathan.d.turner@gmail.com>");
    })
}

#[test]
fn fetches_more_than_one_column_member_path() {
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
            cwd: dirs.test(), h::pipeline(
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
            cwd: dirs.test(), h::pipeline(
            r#"
                open sample.toml
                | get taco
            "#
        ));

        assert!(actual.contains("Unknown column"));
        assert!(actual.contains("did you mean 'taconushell'?"));
    })
}
#[test]
fn errors_fetching_by_index_out_of_bounds_from_table() {
    Playground::setup("get_test_7", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                [spanish_lesson]
                sentence_words = ["Yo", "quiero", "taconushell"]
            "#,
        )]);

        let actual = nu_error!(
            cwd: dirs.test(), h::pipeline(
            r#"
                open sample.toml
                | get spanish_lesson.sentence_words.3
            "#
        ));

        assert!(actual.contains("Row not found"));
        assert!(actual.contains("There isn't a row indexed at '3'"));
        assert!(actual.contains("The table only has 3 rows (0..2)"))
    })
}

#[test]
fn requires_at_least_one_column_member_path() {
    Playground::setup("get_test_8", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("andres.txt")]);

        let actual = nu_error!(
            cwd: dirs.test(), "ls | get"
        );

        assert!(actual.contains("requires member parameter"));
    })
}
