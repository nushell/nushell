use nu_test_support::nu;

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
    let actual =
        nu!(r#"["amigos.txt","arepas.clu","los.txt","tres.txt"] | find arep | to json -r"#);

    assert_eq!(actual.out, r#"["arepas.clu"]"#);
}

#[test]
fn find_with_filepath_search_with_multiple_patterns() {
    let actual =
        nu!(r#"["amigos.txt","arepas.clu","los.txt","tres.txt"] | find arep ami | to json -r"#);

    assert_eq!(actual.out, r#"["amigos.txt","arepas.clu"]"#);
}

#[test]
fn find_takes_into_account_linebreaks_in_string() {
    let actual = nu!(r#""atest\nanothertest\nnohit\n" | find a | length"#);

    assert_eq!(actual.out, "2");
}

#[test]
fn find_with_regex_in_table_keeps_row_if_one_column_matches() {
    let actual = nu!(
        "[[name nickname]; [Maurice moe] [Laurence larry]] | find --regex ce | get name | to json -r"
    );

    assert_eq!(actual.out, r#"["Maurice","Laurence"]"#);
}

#[test]
fn inverted_find_with_regex_in_table_keeps_row_if_none_of_the_columns_matches() {
    let actual = nu!(
        "[[name nickname]; [Maurice moe] [Laurence larry]] | find --regex moe --invert | get name | to json -r"
    );

    assert_eq!(actual.out, r#"["Laurence"]"#);
}

#[test]
fn find_in_table_only_keep_rows_with_matches_on_selected_columns() {
    let actual = nu!(
        "[[name nickname]; [Maurice moe] [Laurence larry]] | find r --columns [nickname] | get name | to json -r"
    );

    assert!(actual.out.contains("Laurence"));
    assert!(!actual.out.contains("Maurice"));
}

#[test]
fn inverted_find_in_table_keeps_row_if_none_of_the_selected_columns_matches() {
    let actual = nu!(
        "[[name nickname]; [Maurice moe] [Laurence larry]] | find r --columns [nickname] --invert | get name | to json -r"
    );

    assert_eq!(actual.out, r#"["Maurice"]"#);
}
