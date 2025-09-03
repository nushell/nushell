use nu_test_support::nu;

#[test]
fn find_with_list_search_with_string() {
    let actual = nu!("[moe larry curly] | find moe | get 0");
    let actual_no_highlight = nu!("[moe larry curly] | find --no-highlight moe | get 0");

    assert_eq!(
        actual.out,
        "\u{1b}[39m\u{1b}[0m\u{1b}[41;39mmoe\u{1b}[0m\u{1b}[39m\u{1b}[0m"
    );
    assert_eq!(actual_no_highlight.out, "moe");
}

#[test]
fn find_with_list_search_with_char() {
    let actual = nu!("[moe larry curly] | find l | to json -r");
    let actual_no_highlight = nu!("[moe larry curly] | find --no-highlight l | to json -r");

    assert_eq!(
        actual.out,
        "[\"\\u001b[39m\\u001b[0m\\u001b[41;39ml\\u001b[0m\\u001b[39marry\\u001b[0m\",\"\\u001b[39mcur\\u001b[0m\\u001b[41;39ml\\u001b[0m\\u001b[39my\\u001b[0m\"]"
    );
    assert_eq!(actual_no_highlight.out, "[\"larry\",\"curly\"]");
}

#[test]
fn find_with_bytestream_search_with_char() {
    let actual = nu!(
        "\"ABC\" | save foo.txt; let res = open foo.txt | find -i abc; rm foo.txt; $res | get 0"
    );
    let actual_no_highlight = nu!(
        "\"ABC\" | save foo.txt; let res = open foo.txt | find -i --no-highlight abc; rm foo.txt; $res | get 0"
    );

    assert_eq!(
        actual.out,
        "\u{1b}[39m\u{1b}[0m\u{1b}[41;39mABC\u{1b}[0m\u{1b}[39m\u{1b}[0m"
    );
    assert_eq!(actual_no_highlight.out, "ABC");
}

#[test]
fn find_with_list_search_with_number() {
    let actual = nu!("[1 2 3 4 5] | find 3 | get 0");
    let actual_no_highlight = nu!("[1 2 3 4 5] | find --no-highlight 3 | get 0");

    assert_eq!(actual.out, "3");
    assert_eq!(actual_no_highlight.out, "3");
}

#[test]
fn find_with_string_search_with_string() {
    let actual = nu!("echo Cargo.toml | find toml");
    let actual_no_highlight = nu!("echo Cargo.toml | find --no-highlight toml");

    assert_eq!(
        actual.out,
        "\u{1b}[39mCargo.\u{1b}[0m\u{1b}[41;39mtoml\u{1b}[0m\u{1b}[39m\u{1b}[0m"
    );
    assert_eq!(actual_no_highlight.out, "Cargo.toml");
}

#[test]
fn find_with_string_search_with_string_not_found() {
    let actual = nu!("[moe larry curly] | find shemp | is-empty");
    let actual_no_highlight = nu!("[moe larry curly] | find --no-highlight shemp | is-empty");

    assert_eq!(actual.out, "true");
    assert_eq!(actual_no_highlight.out, "true");
}

#[test]
fn find_with_filepath_search_with_string() {
    let actual =
        nu!(r#"["amigos.txt","arepas.clu","los.txt","tres.txt"] | find arep | to json -r"#);
    let actual_no_highlight = nu!(
        r#"["amigos.txt","arepas.clu","los.txt","tres.txt"] | find --no-highlight arep | to json -r"#
    );

    assert_eq!(
        actual.out,
        "[\"\\u001b[39m\\u001b[0m\\u001b[41;39marep\\u001b[0m\\u001b[39mas.clu\\u001b[0m\"]"
    );
    assert_eq!(actual_no_highlight.out, "[\"arepas.clu\"]");
}

#[test]
fn find_with_filepath_search_with_multiple_patterns() {
    let actual =
        nu!(r#"["amigos.txt","arepas.clu","los.txt","tres.txt"] | find arep ami | to json -r"#);
    let actual_no_highlight = nu!(
        r#"["amigos.txt","arepas.clu","los.txt","tres.txt"] | find --no-highlight arep ami | to json -r"#
    );

    assert_eq!(
        actual.out,
        "[\"\\u001b[39m\\u001b[0m\\u001b[41;39mami\\u001b[0m\\u001b[39mgos.txt\\u001b[0m\",\"\\u001b[39m\\u001b[0m\\u001b[41;39marep\\u001b[0m\\u001b[39mas.clu\\u001b[0m\"]"
    );
    assert_eq!(actual_no_highlight.out, "[\"amigos.txt\",\"arepas.clu\"]");
}

#[test]
fn find_takes_into_account_linebreaks_in_string() {
    let actual = nu!(r#""atest\nanothertest\nnohit\n" | find a | length"#);
    let actual_no_highlight =
        nu!(r#""atest\nanothertest\nnohit\n" | find --no-highlight a | length"#);

    assert_eq!(actual.out, "2");
    assert_eq!(actual_no_highlight.out, "2");
}

#[test]
fn find_with_regex_in_table_keeps_row_if_one_column_matches() {
    let actual = nu!(
        "[[name nickname]; [Maurice moe] [Laurence larry]] | find --regex ce | get name | to json -r"
    );
    let actual_no_highlight = nu!(
        "[[name nickname]; [Maurice moe] [Laurence larry]] | find --no-highlight --regex ce | get name | to json -r"
    );

    assert_eq!(
        actual.out,
        r#"["\u001b[39mMauri\u001b[0m\u001b[41;39mce\u001b[0m\u001b[39m\u001b[0m","\u001b[39mLauren\u001b[0m\u001b[41;39mce\u001b[0m\u001b[39m\u001b[0m"]"#
    );
    assert_eq!(actual_no_highlight.out, r#"["Maurice","Laurence"]"#);
}

#[test]
fn inverted_find_with_regex_in_table_keeps_row_if_none_of_the_columns_matches() {
    let actual = nu!(
        "[[name nickname]; [Maurice moe] [Laurence larry]] | find --regex moe --invert | get name | to json -r"
    );
    let actual_no_highlight = nu!(
        "[[name nickname]; [Maurice moe] [Laurence larry]] | find --no-highlight --regex moe --invert | get name | to json -r"
    );

    assert_eq!(actual.out, r#"["Laurence"]"#);
    assert_eq!(actual_no_highlight.out, r#"["Laurence"]"#);
}

#[test]
fn find_in_table_only_keep_rows_with_matches_on_selected_columns() {
    let actual = nu!(
        "[[name nickname]; [Maurice moe] [Laurence larry]] | find r --columns [nickname] | get name | to json -r"
    );
    let actual_no_highlight = nu!(
        "[[name nickname]; [Maurice moe] [Laurence larry]] | find r --no-highlight --columns [nickname] | get name | to json -r"
    );

    assert!(actual.out.contains("Laurence"));
    assert!(!actual.out.contains("Maurice"));
    assert!(actual_no_highlight.out.contains("Laurence"));
    assert!(!actual_no_highlight.out.contains("Maurice"));
}

#[test]
fn inverted_find_in_table_keeps_row_if_none_of_the_selected_columns_matches() {
    let actual = nu!(
        "[[name nickname]; [Maurice moe] [Laurence larry]] | find r --columns [nickname] --invert | get name | to json -r"
    );
    let actual_no_highlight = nu!(
        "[[name nickname]; [Maurice moe] [Laurence larry]] | find r --no-highlight --columns [nickname] --invert | get name | to json -r"
    );

    assert_eq!(actual.out, r#"["Maurice"]"#);
    assert_eq!(actual_no_highlight.out, r#"["Maurice"]"#);
}

#[test]
fn find_in_table_keeps_row_with_single_matched_and_keeps_other_columns() {
    let actual = nu!(
        "[[name nickname Age]; [Maurice moe 23] [Laurence larry 67] [William will 18]] | find Maurice"
    );
    let actual_no_highlight = nu!(
        "[[name nickname Age]; [Maurice moe 23] [Laurence larry 67] [William will 18]] | find --no-highlight Maurice"
    );

    println!("{:?}", actual.out);
    assert!(actual.out.contains("moe"));
    assert!(actual.out.contains("Maurice"));
    assert!(actual.out.contains("23"));

    println!("{:?}", actual_no_highlight.out);
    assert!(actual_no_highlight.out.contains("moe"));
    assert!(actual_no_highlight.out.contains("Maurice"));
    assert!(actual_no_highlight.out.contains("23"));
}

#[test]
fn find_in_table_keeps_row_with_multiple_matched_and_keeps_other_columns() {
    let actual = nu!(
        "[[name nickname Age]; [Maurice moe 23] [Laurence larry 67] [William will 18] [William bill 60]] | find moe William"
    );
    let actual_no_highlight = nu!(
        "[[name nickname Age]; [Maurice moe 23] [Laurence larry 67] [William will 18] [William bill 60]] | find --no-highlight moe William"
    );

    println!("{:?}", actual.out);
    assert!(actual.out.contains("moe"));
    assert!(actual.out.contains("Maurice"));
    assert!(actual.out.contains("23"));
    assert!(actual.out.contains("William"));
    assert!(actual.out.contains("will"));
    assert!(actual.out.contains("18"));
    assert!(actual.out.contains("bill"));
    assert!(actual.out.contains("60"));

    println!("{:?}", actual_no_highlight.out);
    assert!(actual_no_highlight.out.contains("moe"));
    assert!(actual_no_highlight.out.contains("Maurice"));
    assert!(actual_no_highlight.out.contains("23"));
    assert!(actual_no_highlight.out.contains("William"));
    assert!(actual_no_highlight.out.contains("will"));
    assert!(actual_no_highlight.out.contains("18"));
    assert!(actual_no_highlight.out.contains("bill"));
    assert!(actual_no_highlight.out.contains("60"));
}

#[test]
fn find_with_string_search_with_special_char_1() {
    let actual = nu!("[[d]; [a?b] [a*b] [a{1}b] ] | find '?' | to json -r");
    let actual_no_highlight =
        nu!("[[d]; [a?b] [a*b] [a{1}b] ] | find --no-highlight '?' | to json -r");

    assert_eq!(
        actual.out,
        "[{\"d\":\"\\u001b[39ma\\u001b[0m\\u001b[41;39m?\\u001b[0m\\u001b[39mb\\u001b[0m\"}]"
    );
    assert_eq!(actual_no_highlight.out, "[{\"d\":\"a?b\"}]");
}

#[test]
fn find_with_string_search_with_special_char_2() {
    let actual = nu!("[[d]; [a?b] [a*b] [a{1}b]] | find '*' | to json -r");
    let actual_no_highlight =
        nu!("[[d]; [a?b] [a*b] [a{1}b]] | find --no-highlight '*' | to json -r");

    assert_eq!(
        actual.out,
        "[{\"d\":\"\\u001b[39ma\\u001b[0m\\u001b[41;39m*\\u001b[0m\\u001b[39mb\\u001b[0m\"}]"
    );
    assert_eq!(actual_no_highlight.out, "[{\"d\":\"a*b\"}]");
}

#[test]
fn find_with_string_search_with_special_char_3() {
    let actual = nu!("[[d]; [a?b] [a*b] [a{1}b] ] | find '{1}' | to json -r");
    let actual_no_highlight =
        nu!("[[d]; [a?b] [a*b] [a{1}b] ] | find --no-highlight '{1}' | to json -r");

    assert_eq!(
        actual.out,
        "[{\"d\":\"\\u001b[39ma\\u001b[0m\\u001b[41;39m{1}\\u001b[0m\\u001b[39mb\\u001b[0m\"}]"
    );
    assert_eq!(actual_no_highlight.out, "[{\"d\":\"a{1}b\"}]");
}

#[test]
fn find_with_string_search_with_special_char_4() {
    let actual = nu!("[{d: a?b} {d: a*b} {d: a{1}b} {d: a[]b}] | find '[' | to json -r");
    let actual_no_highlight =
        nu!("[{d: a?b} {d: a*b} {d: a{1}b} {d: a[]b}] | find --no-highlight '[' | to json -r");

    assert_eq!(
        actual.out,
        "[{\"d\":\"\\u001b[39ma\\u001b[0m\\u001b[41;39m[\\u001b[0m\\u001b[39m]b\\u001b[0m\"}]"
    );
    assert_eq!(actual_no_highlight.out, "[{\"d\":\"a[]b\"}]");
}

#[test]
fn find_with_string_search_with_special_char_5() {
    let actual = nu!("[{d: a?b} {d: a*b} {d: a{1}b} {d: a[]b}] | find ']' | to json -r");
    let actual_no_highlight =
        nu!("[{d: a?b} {d: a*b} {d: a{1}b} {d: a[]b}] | find --no-highlight ']' | to json -r");

    assert_eq!(
        actual.out,
        "[{\"d\":\"\\u001b[39ma[\\u001b[0m\\u001b[41;39m]\\u001b[0m\\u001b[39mb\\u001b[0m\"}]"
    );
    assert_eq!(actual_no_highlight.out, "[{\"d\":\"a[]b\"}]");
}

#[test]
fn find_with_string_search_with_special_char_6() {
    let actual = nu!("[{d: a?b} {d: a*b} {d: a{1}b} {d: a[]b}] | find '[]' | to json -r");
    let actual_no_highlight =
        nu!("[{d: a?b} {d: a*b} {d: a{1}b} {d: a[]b}] | find --no-highlight '[]' | to json -r");

    assert_eq!(
        actual.out,
        "[{\"d\":\"\\u001b[39ma\\u001b[0m\\u001b[41;39m[]\\u001b[0m\\u001b[39mb\\u001b[0m\"}]"
    );
    assert_eq!(actual_no_highlight.out, "[{\"d\":\"a[]b\"}]");
}

#[test]
fn find_in_nested_list_dont_match_bracket() {
    let actual = nu!(r#"[ [foo bar] [foo baz] ] | find "[" | to json -r"#);

    assert_eq!(actual.out, "[]");
}

#[test]
fn find_and_highlight_in_nested_list() {
    let actual = nu!(r#"[ [foo bar] [foo baz] ] | find "foo" | to json -r"#);

    assert_eq!(
        actual.out,
        r#"[["\u001b[39m\u001b[0m\u001b[41;39mfoo\u001b[0m\u001b[39m\u001b[0m","bar"],["\u001b[39m\u001b[0m\u001b[41;39mfoo\u001b[0m\u001b[39m\u001b[0m","baz"]]"#
    );
}
