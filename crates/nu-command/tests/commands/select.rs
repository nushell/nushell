use nu_test_support::fs::Stub::{EmptyFile, FileWithContentToBeTrimmed};
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn regular_columns() {
    let actual = nu!(pipeline(
        r#"
            echo [
                [first_name, last_name, rusty_at, type];

                [Andrés Robalino '10/11/2013' A]
                [JT Turner '10/12/2013' B]
                [Yehuda Katz '10/11/2013' A]
            ]
            | select rusty_at last_name
            | get 0
            | get last_name
        "#
    ));

    assert_eq!(actual.out, "Robalino");
}

#[test]
fn complex_nested_columns() {
    Playground::setup("select_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "los_tres_caballeros.json",
            r#"
                {
                    "nu": {
                        "committers": [
                            {"name": "Andrés N. Robalino"},
                            {"name": "JT Turner"},
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
                | select nu."0xATYKARNU" nu.committers.name nu.releases.version
                | get nu_releases_version
                | where $it > "0.8"
                | get 0
            "#
        ));

        assert_eq!(actual.out, "0.9999999");
    })
}

#[test]
fn fails_if_given_unknown_column_name() {
    let actual = nu!(pipeline(
        r#"
            echo [
                [first_name, last_name, rusty_at, type];

                [Andrés Robalino '10/11/2013' A]
                [JT Turner '10/12/2013' B]
                [Yehuda Katz '10/11/2013' A]
            ]
            | select rrusty_at first_name
            | length
        "#
    ));

    assert!(actual.err.contains("nu::shell::name_not_found"));
}

#[test]
fn column_names_with_spaces() {
    let actual = nu!(pipeline(
        r#"
            echo [
                ["first name", "last name"];

                [Andrés Robalino]
                [Andrés Jnth]
            ]
            | select "last name"
            | get "last name"
            | str join " "
        "#
    ));

    assert_eq!(actual.out, "Robalino Jnth");
}

#[test]
fn ignores_duplicate_columns_selected() {
    let actual = nu!(pipeline(
        r#"
            echo [
                ["first name", "last name"];

                [Andrés Robalino]
                [Andrés Jnth]
            ]
            | select "first name" "last name" "first name"
            | columns
            | str join " "
        "#
    ));

    assert_eq!(actual.out, "first name last name");
}

#[test]
fn selects_a_row() {
    Playground::setup("select_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("notes.txt"), EmptyFile("arepas.txt")]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls
                | sort-by name
                | select 0
                | get name.0
            "#
        ));

        assert_eq!(actual.out, "arepas.txt");
    });
}

#[test]
fn selects_many_rows() {
    Playground::setup("select_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("notes.txt"), EmptyFile("arepas.txt")]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls
                | get name
                | select 1 0
                | length
            "#
        ));

        assert_eq!(actual.out, "2");
    });
}

#[test]
fn select_ignores_errors_successfully1() {
    let actual = nu!("[{a: 1, b: 2} {a: 3, b: 5} {a: 3}] | select b? | length");

    assert_eq!(actual.out, "3".to_string());
    assert!(actual.err.is_empty());
}

#[test]
fn select_ignores_errors_successfully2() {
    let actual = nu!("[{a: 1} {a: 2} {a: 3}] | select b? | to nuon");

    assert_eq!(actual.out, "[[b]; [null], [null], [null]]".to_string());
    assert!(actual.err.is_empty());
}

#[test]
fn select_ignores_errors_successfully3() {
    let actual = nu!("sys | select invalid_key? | to nuon");

    assert_eq!(actual.out, "{invalid_key: null}".to_string());
    assert!(actual.err.is_empty());
}

#[test]
fn select_ignores_errors_successfully4() {
    let actual =
        nu!(r#""key val\na 1\nb 2\n" | lines | split column -c " " | select foo? | to nuon"#);

    assert_eq!(actual.out, r#"[[foo]; [null], [null], [null]]"#.to_string());
    assert!(actual.err.is_empty());
}

#[test]
fn select_failed1() {
    let actual = nu!("[{a: 1, b: 2} {a: 3, b: 5} {a: 3}] | select b ");

    assert!(actual.out.is_empty());
    assert!(actual.err.contains("cannot find column"));
}

#[test]
fn select_failed2() {
    let actual = nu!("[{a: 1} {a: 2} {a: 3}] | select b");

    assert!(actual.out.is_empty());
    assert!(actual.err.contains("cannot find column"));
}

#[test]
fn select_failed3() {
    let actual = nu!(r#""key val\na 1\nb 2\n" | lines | split column -c " " | select "100""#);

    assert!(actual.out.is_empty());
    assert!(actual.err.contains("cannot find column"));
}

#[test]
fn select_failed4() {
    let actual = nu!("[{a: 1 b: 10}, {a:2, b:11}] | select 0 0");

    assert!(actual.err.contains("Select can't get the same row twice"));
}

#[test]
fn ignore_errors_works() {
    let actual = nu!(r#"
        let path = "foo";
        [{}] | select -i $path | to nuon
        "#);

    assert_eq!(actual.out, "[[foo]; [null]]");
}
