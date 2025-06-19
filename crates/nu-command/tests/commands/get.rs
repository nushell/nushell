use nu_test_support::fs::Stub::FileWithContent;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn simple_get_record() {
    let actual = nu!(r#"({foo: 'bar'} | get foo) == "bar""#);
    assert_eq!(actual.out, "true");
}

#[test]
fn simple_get_list() {
    let actual = nu!(r#"([{foo: 'bar'}] | get foo) == [bar]"#);
    assert_eq!(actual.out, "true");
}

#[test]
fn fetches_a_row() {
    Playground::setup("get_test_1", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "sample.toml",
            r#"
                nu_party_venue = "zion"
            "#,
        )]);

        let actual = nu!( cwd: dirs.test(), "open sample.toml | get nu_party_venue");

        assert_eq!(actual.out, "zion");
    })
}

#[test]
fn fetches_by_index() {
    Playground::setup("get_test_2", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "sample.toml",
            r#"
                [package]
                name = "nu"
                version = "0.4.1"
                authors = ["Yehuda Katz <wycats@gmail.com>", "JT Turner <547158+jntrnr@users.noreply.github.com>", "Andrés N. Robalino <andres@androbtech.com>"]
                description = "When arepas shells are tasty and fun."
            "#,
        )]);

        let actual = nu!( cwd: dirs.test(), "open sample.toml | get package.authors.2");

        assert_eq!(actual.out, "Andrés N. Robalino <andres@androbtech.com>");
    })
}

#[test]
fn fetches_by_column_path() {
    Playground::setup("get_test_3", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "sample.toml",
            r#"
                [package]
                name = "nu"
            "#,
        )]);

        let actual = nu!( cwd: dirs.test(), "open sample.toml | get package.name");

        assert_eq!(actual.out, "nu");
    })
}

#[test]
fn column_paths_are_either_double_quoted_or_regular_unquoted_words_separated_by_dot() {
    Playground::setup("get_test_4", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "sample.toml",
            r#"
                [package]
                9999 = ["Yehuda Katz <wycats@gmail.com>", "JT Turner <jtd.turner@gmail.com>", "Andrés N. Robalino <andres@androbtech.com>"]
                description = "When arepas shells are tasty and fun."
            "#,
        )]);

        let actual = nu!( cwd: dirs.test(), r#"open sample.toml | get package."9999" | length"#);

        assert_eq!(actual.out, "3");
    })
}

#[test]
fn fetches_more_than_one_column_path() {
    Playground::setup("get_test_5", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "sample.toml",
            r#"
                [[fortune_tellers]]
                name = "Andrés N. Robalino"
                arepas = 1

                [[fortune_tellers]]
                name = "JT"
                arepas = 1

                [[fortune_tellers]]
                name = "Yehuda Katz"
                arepas = 1
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            "
                open sample.toml
                | get fortune_tellers.2.name fortune_tellers.0.name fortune_tellers.1.name
                | get 2
            "
        ));

        assert_eq!(actual.out, "JT");
    })
}

#[test]
fn errors_fetching_by_column_not_present() {
    Playground::setup("get_test_6", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "sample.toml",
            r#"
                [tacos]
                sentence_words = ["Yo", "quiero", "tacos"]
                [pizzanushell]
                sentence-words = ["I", "want", "pizza"]
            "#,
        )]);

        let actual = nu!( cwd: dirs.test(), "open sample.toml | get taco");

        assert!(actual.err.contains("Name not found"),);
        assert!(actual.err.contains("did you mean 'tacos'"),);
    })
}

#[test]
fn errors_fetching_by_column_using_a_number() {
    Playground::setup("get_test_7", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "sample.toml",
            r#"
                [spanish_lesson]
                0 = "can only be fetched with 0 double quoted."
            "#,
        )]);

        let actual = nu!( cwd: dirs.test(), "open sample.toml | get spanish_lesson.0");

        assert!(actual.err.contains("Type mismatch"),);
    })
}

#[test]
fn errors_fetching_by_index_out_of_bounds() {
    Playground::setup("get_test_8", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "sample.toml",
            r#"
                [spanish_lesson]
                sentence_words = ["Yo", "quiero", "taconushell"]
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), " open sample.toml | get spanish_lesson.sentence_words.3 ");

        assert!(actual.err.contains("Row number too large (max: 2)"),);
        assert!(actual.err.contains("too large"),);
    })
}

#[test]
fn errors_fetching_by_accessing_empty_list() {
    let actual = nu!("[] | get 3");
    assert!(actual.err.contains("Row number too large (empty content)"),);
}

#[test]
fn quoted_column_access() {
    let actual = nu!(r#"'[{"foo bar": {"baz": 4}}]' | from json | get "foo bar".baz.0 "#);

    assert_eq!(actual.out, "4");
}

#[test]
fn get_does_not_delve_too_deep_in_nested_lists() {
    let actual = nu!("[[{foo: bar}]] | get foo");

    assert!(actual.err.contains("cannot find column"));
}

#[test]
fn ignore_errors_works() {
    let actual = nu!(r#" let path = "foo"; {} | get -o $path | to nuon "#);

    assert_eq!(actual.out, "null");
}

#[test]
fn ignore_multiple() {
    let actual = nu!(r#"[[a];[b]] | get -o c d | to nuon"#);

    assert_eq!(actual.out, "[[null], [null]]");
}

#[test]
fn test_const() {
    let actual = nu!(r#"const x = [1 2 3] | get 1; $x"#);
    assert_eq!(actual.out, "2");
}
