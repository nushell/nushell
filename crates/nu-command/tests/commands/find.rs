use nu_test_support::nu;

#[test]
fn find_with_list_search_with_string() {
    let actual = nu!("[moe larry curly] | find moe | get 0");

    assert_eq!(
        actual.out,
        "\u{1b}[37m\u{1b}[0m\u{1b}[41;37mmoe\u{1b}[0m\u{1b}[37m\u{1b}[0m"
    );
}

#[test]
fn find_with_list_search_with_char() {
    let actual = nu!("[moe larry curly] | find l | to json -r");

    assert_eq!(actual.out, "[\"\\u001b[37m\\u001b[0m\\u001b[41;37ml\\u001b[0m\\u001b[37marry\\u001b[0m\",\"\\u001b[37mcur\\u001b[0m\\u001b[41;37ml\\u001b[0m\\u001b[37my\\u001b[0m\"]");
}

#[test]
fn find_with_bytestream_search_with_char() {
    let actual =
        nu!("\"ABC\" | save foo.txt; let res = open foo.txt | find abc; rm foo.txt; $res | get 0");
    assert_eq!(
        actual.out,
        "\u{1b}[37m\u{1b}[0m\u{1b}[41;37mABC\u{1b}[0m\u{1b}[37m\u{1b}[0m"
    )
}

#[test]
fn find_with_list_search_with_number() {
    let actual = nu!("[1 2 3 4 5] | find 3 | get 0");

    assert_eq!(actual.out, "3");
}

#[test]
fn find_with_string_search_with_string() {
    let actual = nu!("echo Cargo.toml | find toml");

    assert_eq!(
        actual.out,
        "\u{1b}[37mCargo.\u{1b}[0m\u{1b}[41;37mtoml\u{1b}[0m\u{1b}[37m\u{1b}[0m"
    );
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

    assert_eq!(
        actual.out,
        "[\"\\u001b[37m\\u001b[0m\\u001b[41;37marep\\u001b[0m\\u001b[37mas.clu\\u001b[0m\"]"
    );
}

#[test]
fn find_with_filepath_search_with_multiple_patterns() {
    let actual =
        nu!(r#"["amigos.txt","arepas.clu","los.txt","tres.txt"] | find arep ami | to json -r"#);

    assert_eq!(actual.out, "[\"\\u001b[37m\\u001b[0m\\u001b[41;37mami\\u001b[0m\\u001b[37mgos.txt\\u001b[0m\",\"\\u001b[37m\\u001b[0m\\u001b[41;37marep\\u001b[0m\\u001b[37mas.clu\\u001b[0m\"]");
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

#[test]
fn find_in_table_keeps_row_with_single_matched_and_keeps_other_columns() {
    let actual = nu!("[[name nickname Age]; [Maurice moe 23] [Laurence larry 67] [William will 18]] | find Maurice");

    println!("{:?}", actual.out);
    assert!(actual.out.contains("moe"));
    assert!(actual.out.contains("Maurice"));
    assert!(actual.out.contains("23"));
}

#[test]
fn find_in_table_keeps_row_with_multiple_matched_and_keeps_other_columns() {
    let actual = nu!("[[name nickname Age]; [Maurice moe 23] [Laurence larry 67] [William will 18] [William bill 60]] | find moe William");

    println!("{:?}", actual.out);
    assert!(actual.out.contains("moe"));
    assert!(actual.out.contains("Maurice"));
    assert!(actual.out.contains("23"));
    assert!(actual.out.contains("William"));
    assert!(actual.out.contains("will"));
    assert!(actual.out.contains("18"));
    assert!(actual.out.contains("bill"));
    assert!(actual.out.contains("60"));
}
