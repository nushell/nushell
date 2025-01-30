use nu_test_support::nu;

#[test]
fn find_with_list_search_with_string() {
    let actual = nu!("[moe larry curly] | find moe | get 0");
    let actual_raw = nu!("[moe larry curly] | find --raw moe | get 0");

    assert_eq!(
        actual.out,
        "\u{1b}[37m\u{1b}[0m\u{1b}[41;37mmoe\u{1b}[0m\u{1b}[37m\u{1b}[0m"
    );
    assert_eq!(
        actual_raw.out,
        "moe"
    );
}

#[test]
fn find_with_list_search_with_char() {
    let actual = nu!("[moe larry curly] | find l | to json -r");
    let actual_raw = nu!("[moe larry curly] | find --raw l | to json -r");

    assert_eq!(actual.out, "[\"\\u001b[37m\\u001b[0m\\u001b[41;37ml\\u001b[0m\\u001b[37marry\\u001b[0m\",\"\\u001b[37mcur\\u001b[0m\\u001b[41;37ml\\u001b[0m\\u001b[37my\\u001b[0m\"]");
    assert_eq!(actual_raw.out, "[\"larry\",\"curly\"]");
}

#[test]
fn find_with_bytestream_search_with_char() {
    let actual =
        nu!("\"ABC\" | save foo.txt; let res = open foo.txt | find abc; rm foo.txt; $res | get 0");
    let actual_raw =
        nu!("\"ABC\" | save foo.txt; let res = open foo.txt | find --raw abc; rm foo.txt; $res | get 0");

    assert_eq!(
        actual.out,
        "\u{1b}[37m\u{1b}[0m\u{1b}[41;37mABC\u{1b}[0m\u{1b}[37m\u{1b}[0m"
    );
    assert_eq!(
        actual_raw.out,
        "ABC"
    );
}

#[test]
fn find_with_list_search_with_number() {
    let actual = nu!("[1 2 3 4 5] | find 3 | get 0");
    let actual_raw = nu!("[1 2 3 4 5] | find --raw 3 | get 0");

    assert_eq!(actual.out, "3");
    assert_eq!(actual_raw.out, "3");
}

#[test]
fn find_with_string_search_with_string() {
    let actual = nu!("echo Cargo.toml | find toml");
    let actual_raw = nu!("echo Cargo.toml | find --raw toml");

    assert_eq!(
        actual.out,
        "\u{1b}[37mCargo.\u{1b}[0m\u{1b}[41;37mtoml\u{1b}[0m\u{1b}[37m\u{1b}[0m"
    );
    assert_eq!(
        actual_raw.out,
        "Cargo.toml"
    );
}

#[test]
fn find_with_string_search_with_string_not_found() {
    let actual = nu!("[moe larry curly] | find shemp | is-empty");
    let actual_raw = nu!("[moe larry curly] | find --raw shemp | is-empty");

    assert_eq!(actual.out, "true");
    assert_eq!(actual_raw.out, "true");
}

#[test]
fn find_with_filepath_search_with_string() {
    let actual =
        nu!(r#"["amigos.txt","arepas.clu","los.txt","tres.txt"] | find arep | to json -r"#);
    let actual_raw =
        nu!(r#"["amigos.txt","arepas.clu","los.txt","tres.txt"] | find --raw arep | to json -r"#);

    assert_eq!(
        actual.out,
        "[\"\\u001b[37m\\u001b[0m\\u001b[41;37marep\\u001b[0m\\u001b[37mas.clu\\u001b[0m\"]"
    );
    assert_eq!(
        actual_raw.out,
        "[\"arepas.clu\"]"
    );
}

#[test]
fn find_with_filepath_search_with_multiple_patterns() {
    let actual =
        nu!(r#"["amigos.txt","arepas.clu","los.txt","tres.txt"] | find arep ami | to json -r"#);
    let actual_raw =
        nu!(r#"["amigos.txt","arepas.clu","los.txt","tres.txt"] | find --raw arep ami | to json -r"#);

    assert_eq!(actual.out, "[\"\\u001b[37m\\u001b[0m\\u001b[41;37mami\\u001b[0m\\u001b[37mgos.txt\\u001b[0m\",\"\\u001b[37m\\u001b[0m\\u001b[41;37marep\\u001b[0m\\u001b[37mas.clu\\u001b[0m\"]");
    assert_eq!(actual_raw.out, "[\"amigos.txt\",\"arepas.clu\"]");
}

#[test]
fn find_takes_into_account_linebreaks_in_string() {
    let actual = nu!(r#""atest\nanothertest\nnohit\n" | find a | length"#);
    let actual_raw = nu!(r#""atest\nanothertest\nnohit\n" | find --raw a | length"#);

    assert_eq!(actual.out, "2");
    assert_eq!(actual_raw.out, "2");
}

#[test]
fn find_with_regex_in_table_keeps_row_if_one_column_matches() {
    let actual = nu!(
        "[[name nickname]; [Maurice moe] [Laurence larry]] | find --regex ce | get name | to json -r"
    );
    let actual_raw = nu!(
        "[[name nickname]; [Maurice moe] [Laurence larry]] | find --raw --regex ce | get name | to json -r"
    );

    assert_eq!(actual.out, r#"["Maurice","Laurence"]"#);
    assert_eq!(actual_raw.out, r#"["Maurice","Laurence"]"#);
}

#[test]
fn inverted_find_with_regex_in_table_keeps_row_if_none_of_the_columns_matches() {
    let actual = nu!(
        "[[name nickname]; [Maurice moe] [Laurence larry]] | find --regex moe --invert | get name | to json -r"
    );
    let actual_raw = nu!(
        "[[name nickname]; [Maurice moe] [Laurence larry]] | find --raw --regex moe --invert | get name | to json -r"
    );

    assert_eq!(actual.out, r#"["Laurence"]"#);
    assert_eq!(actual_raw.out, r#"["Laurence"]"#);
}

#[test]
fn find_in_table_only_keep_rows_with_matches_on_selected_columns() {
    let actual = nu!(
        "[[name nickname]; [Maurice moe] [Laurence larry]] | find r --columns [nickname] | get name | to json -r"
    );
    let actual_raw = nu!(
        "[[name nickname]; [Maurice moe] [Laurence larry]] | find r --raw --columns [nickname] | get name | to json -r"
    );

    assert!(actual.out.contains("Laurence"));
    assert!(!actual.out.contains("Maurice"));
    assert!(actual_raw.out.contains("Laurence"));
    assert!(!actual_raw.out.contains("Maurice"));
}

#[test]
fn inverted_find_in_table_keeps_row_if_none_of_the_selected_columns_matches() {
    let actual = nu!(
        "[[name nickname]; [Maurice moe] [Laurence larry]] | find r --columns [nickname] --invert | get name | to json -r"
    );
    let actual_raw = nu!(
        "[[name nickname]; [Maurice moe] [Laurence larry]] | find r --raw --columns [nickname] --invert | get name | to json -r"
    );

    assert_eq!(actual.out, r#"["Maurice"]"#);
    assert_eq!(actual_raw.out, r#"["Maurice"]"#);
}

#[test]
fn find_in_table_keeps_row_with_single_matched_and_keeps_other_columns() {
    let actual = nu!("[[name nickname Age]; [Maurice moe 23] [Laurence larry 67] [William will 18]] | find Maurice");
    let actual_raw = nu!("[[name nickname Age]; [Maurice moe 23] [Laurence larry 67] [William will 18]] | find --raw Maurice");

    println!("{:?}", actual.out);
    assert!(actual.out.contains("moe"));
    assert!(actual.out.contains("Maurice"));
    assert!(actual.out.contains("23"));

    println!("{:?}", actual_raw.out);
    assert!(actual_raw.out.contains("moe"));
    assert!(actual_raw.out.contains("Maurice"));
    assert!(actual_raw.out.contains("23"));
}

#[test]
fn find_in_table_keeps_row_with_multiple_matched_and_keeps_other_columns() {
    let actual = nu!("[[name nickname Age]; [Maurice moe 23] [Laurence larry 67] [William will 18] [William bill 60]] | find moe William");
    let actual_raw = nu!("[[name nickname Age]; [Maurice moe 23] [Laurence larry 67] [William will 18] [William bill 60]] | find --raw moe William");

    println!("{:?}", actual.out);
    assert!(actual.out.contains("moe"));
    assert!(actual.out.contains("Maurice"));
    assert!(actual.out.contains("23"));
    assert!(actual.out.contains("William"));
    assert!(actual.out.contains("will"));
    assert!(actual.out.contains("18"));
    assert!(actual.out.contains("bill"));
    assert!(actual.out.contains("60"));

    println!("{:?}", actual_raw.out);
    assert!(actual_raw.out.contains("moe"));
    assert!(actual_raw.out.contains("Maurice"));
    assert!(actual_raw.out.contains("23"));
    assert!(actual_raw.out.contains("William"));
    assert!(actual_raw.out.contains("will"));
    assert!(actual_raw.out.contains("18"));
    assert!(actual_raw.out.contains("bill"));
    assert!(actual_raw.out.contains("60"));
}

#[test]
fn find_with_string_search_with_special_char_1() {
    let actual = nu!("[[d]; [a?b] [a*b] [a{1}b] ] | find '?' | to json -r");
    let actual_raw = nu!("[[d]; [a?b] [a*b] [a{1}b] ] | find --raw '?' | to json -r");

    assert_eq!(
        actual.out,
        "[{\"d\":\"\\u001b[37ma\\u001b[0m\\u001b[41;37m?\\u001b[0m\\u001b[37mb\\u001b[0m\"}]"
    );
    assert_eq!(
        actual_raw.out,
        "[{\"d\":\"a?b\"}]"
    );
}

#[test]
fn find_with_string_search_with_special_char_2() {
    let actual = nu!("[[d]; [a?b] [a*b] [a{1}b]] | find '*' | to json -r");
    let actual_raw = nu!("[[d]; [a?b] [a*b] [a{1}b]] | find --raw '*' | to json -r");

    assert_eq!(
        actual.out,
        "[{\"d\":\"\\u001b[37ma\\u001b[0m\\u001b[41;37m*\\u001b[0m\\u001b[37mb\\u001b[0m\"}]"
    );
    assert_eq!(
        actual_raw.out,
        "[{\"d\":\"a*b\"}]"
    );
}

#[test]
fn find_with_string_search_with_special_char_3() {
    let actual = nu!("[[d]; [a?b] [a*b] [a{1}b] ] | find '{1}' | to json -r");
    let actual_raw = nu!("[[d]; [a?b] [a*b] [a{1}b] ] | find --raw '{1}' | to json -r");

    assert_eq!(
        actual.out,
        "[{\"d\":\"\\u001b[37ma\\u001b[0m\\u001b[41;37m{1}\\u001b[0m\\u001b[37mb\\u001b[0m\"}]"
    );
    assert_eq!(
        actual_raw.out,
        "[{\"d\":\"a{1}b\"}]"
    );
}

#[test]
fn find_with_string_search_with_special_char_4() {
    let actual = nu!("[{d: a?b} {d: a*b} {d: a{1}b} {d: a[]b}] | find '[' | to json -r");
    let actual_raw = nu!("[{d: a?b} {d: a*b} {d: a{1}b} {d: a[]b}] | find --raw '[' | to json -r");

    assert_eq!(
        actual.out,
        "[{\"d\":\"\\u001b[37ma\\u001b[0m\\u001b[41;37m[\\u001b[0m\\u001b[37m]b\\u001b[0m\"}]"
    );
    assert_eq!(
        actual_raw.out,
        "[{\"d\":\"a[]b\"}]"
    );
}

#[test]
fn find_with_string_search_with_special_char_5() {
    let actual = nu!("[{d: a?b} {d: a*b} {d: a{1}b} {d: a[]b}] | find ']' | to json -r");
    let actual_raw = nu!("[{d: a?b} {d: a*b} {d: a{1}b} {d: a[]b}] | find --raw ']' | to json -r");

    assert_eq!(
        actual.out,
        "[{\"d\":\"\\u001b[37ma[\\u001b[0m\\u001b[41;37m]\\u001b[0m\\u001b[37mb\\u001b[0m\"}]"
    );
    assert_eq!(
        actual_raw.out,
        "[{\"d\":\"a[]b\"}]"
    );
}

#[test]
fn find_with_string_search_with_special_char_6() {
    let actual = nu!("[{d: a?b} {d: a*b} {d: a{1}b} {d: a[]b}] | find '[]' | to json -r");
    let actual_raw = nu!("[{d: a?b} {d: a*b} {d: a{1}b} {d: a[]b}] | find --raw '[]' | to json -r");

    assert_eq!(
        actual.out,
        "[{\"d\":\"\\u001b[37ma\\u001b[0m\\u001b[41;37m[]\\u001b[0m\\u001b[37mb\\u001b[0m\"}]"
    );
    assert_eq!(
        actual_raw.out,
        "[{\"d\":\"a[]b\"}]"
    );
}
