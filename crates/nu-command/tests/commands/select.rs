use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::nu;
use nu_test_support::playground::Playground;

#[test]
fn regular_columns() {
    let actual = nu!(r#"
        echo [
            [first_name, last_name, rusty_at, type];
    
            [Andrés Robalino '10/11/2013' A]
            [JT Turner '10/12/2013' B]
            [Yehuda Katz '10/11/2013' A]
        ]
        | select rusty_at last_name
        | get 0
        | get last_name
    "#);

    assert_eq!(actual.out, "Robalino");
}

#[test]
fn complex_nested_columns() {
    let sample = r#"{
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
    }"#;

    let actual = nu!(format!(
        r#"
            {sample}
            | select nu."0xATYKARNU" nu.committers.name nu.releases.version
            | get "nu.releases.version"
            | where $it > "0.8"
            | get 0
        "#
    ));

    assert_eq!(actual.out, "0.9999999");
}

#[test]
fn fails_if_given_unknown_column_name() {
    let actual = nu!(r#"
        [
            [first_name, last_name, rusty_at, type];
    
            [Andrés Robalino '10/11/2013' A]
            [JT Turner '10/12/2013' B]
            [Yehuda Katz '10/11/2013' A]
        ]
        | select rrusty_at first_name
    "#);

    assert!(actual.err.contains("nu::shell::name_not_found"));
}

#[test]
fn column_names_with_spaces() {
    let actual = nu!(r#"
        echo [
            ["first name", "last name"];
    
            [Andrés Robalino]
            [Andrés Jnth]
        ]
        | select "last name"
        | get "last name"
        | str join " "
    "#);

    assert_eq!(actual.out, "Robalino Jnth");
}

#[test]
fn ignores_duplicate_columns_selected() {
    let actual = nu!(r#"
        echo [
            ["first name", "last name"];
    
            [Andrés Robalino]
            [Andrés Jnth]
        ]
        | select "first name" "last name" "first name"
        | columns
        | str join " "
    "#);

    assert_eq!(actual.out, "first name last name");
}

#[test]
fn selects_a_row() {
    Playground::setup("select_test_1", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("notes.txt"), EmptyFile("arepas.txt")]);

        let actual = nu!(cwd: dirs.test(), "
            ls
            | sort-by name
            | select 0
            | get name.0
        ");

        assert_eq!(actual.out, "arepas.txt");
    });
}

#[test]
fn selects_large_row_number() {
    let actual = nu!("seq 1 5 | select 9999999999 | to nuon");
    assert_eq!(actual.out, "[]");
}

#[test]
fn selects_many_rows() {
    Playground::setup("select_test_2", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("notes.txt"), EmptyFile("arepas.txt")]);

        let actual = nu!(cwd: dirs.test(), "
            ls
            | get name
            | select 1 0
            | length
        ");

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
    let actual = nu!("{foo: bar} | select invalid_key? | to nuon");

    assert_eq!(actual.out, "{invalid_key: null}".to_string());
    assert!(actual.err.is_empty());
}

#[test]
fn select_ignores_errors_successfully4() {
    let actual = nu!(
        r#""key val\na 1\nb 2\n" | lines | split column --collapse-empty " " | select foo? | to nuon"#
    );

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
    let actual =
        nu!(r#""key val\na 1\nb 2\n" | lines | split column --collapse-empty " " | select "100""#);

    assert!(actual.out.is_empty());
    assert!(actual.err.contains("cannot find column"));
}

#[test]
fn select_repeated_rows() {
    let actual = nu!("[[a b c]; [1 2 3] [4 5 6] [7 8 9]] | select 0 0 | to nuon");

    assert_eq!(actual.out, "[[a, b, c]; [1, 2, 3]]");
}

#[test]
fn select_repeated_column() {
    let actual = nu!("[[a b c]; [1 2 3] [4 5 6] [7 8 9]] | select a a | to nuon");

    assert_eq!(actual.out, "[[a]; [1], [4], [7]]");
}

#[test]
fn ignore_errors_works() {
    let actual = nu!(r#"
        let path = "foo";
        [{}] | select -o $path | to nuon
        "#);

    assert_eq!(actual.out, "[[foo]; [null]]");
}

#[test]
fn select_on_empty_list_returns_empty_list() {
    // once with a List
    let actual = nu!("[] | select foo | to nuon");
    assert_eq!(actual.out, "[]");

    // and again with a ListStream
    let actual = nu!("[] | each {|i| $i} | select foo | to nuon");
    assert_eq!(actual.out, "[]");
}

#[test]
fn select_columns_with_list_spread() {
    let actual = nu!(r#"
        let columns = [a c];
        echo [[a b c]; [1 2 3]] | select ...$columns | to nuon
        "#);

    assert_eq!(actual.out, "[[a, c]; [1, 3]]");
}

#[test]
fn select_rows_with_list_spread() {
    let actual = nu!(r#"
        let rows = [0 2];
        echo [[a b c]; [1 2 3] [4 5 6] [7 8 9]] | select ...$rows | to nuon
        "#);

    assert_eq!(actual.out, "[[a, b, c]; [1, 2, 3], [7, 8, 9]]");
}

#[test]
fn select_single_row_with_variable() {
    let actual = nu!("let idx = 2; [{a: 1, b: 2} {a: 3, b: 5} {a: 3}] | select $idx | to nuon");

    assert_eq!(actual.out, "[[a]; [3]]".to_string());
    assert!(actual.err.is_empty());
}

#[test]
fn select_with_negative_number_errors_out() {
    let actual = nu!("[1 2 3] | select (-2)");
    assert!(actual.err.contains("negative number"));
}
