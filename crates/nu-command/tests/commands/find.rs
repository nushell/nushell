use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::nu;
use nu_test_support::playground::Playground;

#[test]
fn find_with_list_search_with_string() {
    let actual = nu!("[moe larry curly] | find moe | get 0");

    assert_eq!(actual.out, "moe");
}

#[test]
fn find_with_list_search_with_char() {
    let actual = nu!("[moe larry curly] | find l | to json -r");

    assert_eq!(actual.out, r#"["larry","curly"]"#);
}

#[test]
fn find_with_list_search_with_number() {
    let actual = nu!("[1 2 3 4 5] | find 3 | get 0");

    assert_eq!(actual.out, "3");
}

#[test]
fn find_with_string_search_with_string() {
    let actual = nu!("echo Cargo.toml | find toml");

    assert_eq!(actual.out, "Cargo.toml");
}

#[test]
fn find_with_string_search_with_string_not_found() {
    let actual = nu!("[moe larry curly] | find shemp | is-empty");

    assert_eq!(actual.out, "true");
}

#[test]
fn find_with_filepath_search_with_string() {
    Playground::setup("filepath_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
        ]);

        let actual = nu!(
            cwd: dirs.test(),
            "ls | get name | find arep | to json -r"
        );

        assert_eq!(actual.out, r#"["arepas.clu"]"#);
    })
}

#[test]
fn find_with_filepath_search_with_multiple_patterns() {
    Playground::setup("filepath_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
        ]);

        let actual = nu!(
            cwd: dirs.test(),
            "ls | get name | find arep ami | to json -r"
        );

        assert_eq!(actual.out, r#"["amigos.txt","arepas.clu"]"#);
    })
}

#[test]
fn find_takes_into_account_linebreaks_in_string() {
    let actual = nu!(r#""atest\nanothertest\nnohit\n" | find a | length"#);

    assert_eq!(actual.out, "2");
}

#[test]
fn find_with_regex_in_table_keeps_row_if_one_column_matches() {
    Playground::setup("filepath_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
        ]);

        let actual = nu!(
            cwd: dirs.test(),
            "ls | find --regex t.t | get name | to json -r"
        );

        assert_eq!(actual.out, r#"["amigos.txt","los.txt","tres.txt"]"#);
    })
}

#[test]
fn inverted_find_with_regex_in_table_keeps_row_if_none_of_the_columns_matches() {
    Playground::setup("filepath_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
        ]);

        let actual = nu!(
            cwd: dirs.test(),
            "ls | find --regex t.t --invert | get name | to json -r"
        );

        assert_eq!(actual.out, r#"["arepas.clu"]"#);
    })
}

#[test]
fn find_in_table_only_keep_rows_with_matches_on_selected_columns() {
    Playground::setup("filepath_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("file.txt"),
            EmptyFile("arepas.clu"),
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
        ]);

        let actual = nu!(
            cwd: dirs.test(),
            "ls | find file --columns [name] | get name | to json -r"
        );

        assert!(actual.out.contains("file"));
        assert!(!actual.out.contains("arepas"));
        assert!(!actual.out.contains("los"));
        assert!(!actual.out.contains("tres"));
    })
}

#[test]
fn inverted_find_in_table_keeps_row_if_none_of_the_selected_columns_matches() {
    Playground::setup("filepath_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("file.txt"),
            EmptyFile("arepas.clu"),
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
        ]);

        let actual = nu!(
            cwd: dirs.test(),
            "ls | find file --columns [name] --invert | get name | to json -r"
        );

        assert_eq!(actual.out, r#"["arepas.clu","los.txt","tres.txt"]"#);
    })
}
