use nu_test_support::{fs::Stub::FileWithContent, prelude::*};
use rstest::rstest;

#[rstest]
#[case::highlighted(
    "moe",
    "\u{1b}[39m\u{1b}[0m\u{1b}[41;39mmoe\u{1b}[0m\u{1b}[39m\u{1b}[0m"
)]
#[case::plain("--no-highlight moe", "moe")]
fn find_with_list_search_with_string(
    #[case] find_args: &str,
    #[case] expected: impl IntoValue,
) -> Result {
    let code = format!("[moe larry curly] | find {find_args} | get 0");
    test().run(code).expect_value_eq(expected)
}

#[rstest]
#[case::highlighted(
    "l",
    [
        "\u{1b}[39m\u{1b}[0m\u{1b}[41;39ml\u{1b}[0m\u{1b}[39marry\u{1b}[0m",
        "\u{1b}[39mcur\u{1b}[0m\u{1b}[41;39ml\u{1b}[0m\u{1b}[39my\u{1b}[0m",
    ]
)]
#[case::plain("--no-highlight l", ["larry", "curly"])]
fn find_with_list_search_with_char(
    #[case] find_args: &str,
    #[case] expected: impl IntoValue,
) -> Result {
    let code = format!("[moe larry curly] | find {find_args}");
    test().run(code).expect_value_eq(expected)
}

#[rstest]
#[case::highlighted(
    "-i abc",
    "\u{1b}[39m\u{1b}[0m\u{1b}[41;39mABC\u{1b}[0m\u{1b}[39m\u{1b}[0m"
)]
#[case::plain("-i --no-highlight abc", "ABC")]
fn find_with_bytestream_search_with_char(
    #[ignore] playground: Playground,
    #[case] find_args: &str,
    #[case] expected: impl IntoValue,
) -> Result {
    playground.file("foo.txt", "ABC")?;
    let code = format!("open foo.txt | find {find_args} | get 0");
    test()
        .cwd(playground.path())
        .run(code)
        .expect_value_eq(expected)
}

#[rstest]
#[case::highlighted("3", 3)]
#[case::plain("--no-highlight 3", 3)]
fn find_with_list_search_with_number(
    #[case] find_args: &str,
    #[case] expected: impl IntoValue,
) -> Result {
    let code = format!("[1 2 3 4 5] | find {find_args} | get 0");
    test().run(code).expect_value_eq(expected)
}

#[rstest]
#[case::highlighted(
    "toml",
    "\u{1b}[39mCargo.\u{1b}[0m\u{1b}[41;39mtoml\u{1b}[0m\u{1b}[39m\u{1b}[0m"
)]
#[case::plain("--no-highlight toml", "Cargo.toml")]
fn find_with_string_search_with_string(
    #[case] find_args: &str,
    #[case] expected: impl IntoValue,
) -> Result {
    let code = format!("'Cargo.toml' | find {find_args}");
    test().run(code).expect_value_eq(expected)
}

#[rstest]
#[case::highlighted("shemp", true)]
#[case::plain("--no-highlight shemp", true)]
fn find_with_string_search_with_string_not_found(
    #[case] find_args: &str,
    #[case] expected: impl IntoValue,
) -> Result {
    let code = format!("[moe larry curly] | find {find_args} | is-empty");
    test().run(code).expect_value_eq(expected)
}

#[rstest]
#[case::highlighted(
    "arep",
    ["\u{1b}[39m\u{1b}[0m\u{1b}[41;39marep\u{1b}[0m\u{1b}[39mas.clu\u{1b}[0m"]
)]
#[case::plain("--no-highlight arep", ["arepas.clu"])]
fn find_with_filepath_search_with_string(
    #[case] find_args: &str,
    #[case] expected: impl IntoValue,
) -> Result {
    let code = format!(r#"["amigos.txt", "arepas.clu", "los.txt", "tres.txt"] | find {find_args}"#);
    test().run(code).expect_value_eq(expected)
}

#[rstest]
#[case::highlighted(
    "arep ami",
    [
        "\u{1b}[39m\u{1b}[0m\u{1b}[41;39mami\u{1b}[0m\u{1b}[39mgos.txt\u{1b}[0m",
        "\u{1b}[39m\u{1b}[0m\u{1b}[41;39marep\u{1b}[0m\u{1b}[39mas.clu\u{1b}[0m",
    ]
)]
#[case::plain("--no-highlight arep ami", ["amigos.txt", "arepas.clu"])]
fn find_with_filepath_search_with_multiple_patterns(
    #[case] find_args: &str,
    #[case] expected: impl IntoValue,
) -> Result {
    let code = format!(r#"["amigos.txt", "arepas.clu", "los.txt", "tres.txt"] | find {find_args}"#);
    test().run(code).expect_value_eq(expected)
}

#[rstest]
#[case::highlighted("a", 2)]
#[case::plain("--no-highlight a", 2)]
fn find_takes_into_account_linebreaks_in_string(
    #[case] find_args: &str,
    #[case] expected: impl IntoValue,
) -> Result {
    let code = format!(r#""atest\nanothertest\nnohit\n" | find {find_args} | length"#);
    test().run(code).expect_value_eq(expected)
}

#[rstest]
#[case::highlighted(
    "--regex ce",
    [
        "\u{1b}[39mMauri\u{1b}[0m\u{1b}[41;39mce\u{1b}[0m\u{1b}[39m\u{1b}[0m",
        "\u{1b}[39mLauren\u{1b}[0m\u{1b}[41;39mce\u{1b}[0m\u{1b}[39m\u{1b}[0m",
    ]
)]
#[case::plain(
    "--no-highlight --regex ce",
    ["Maurice", "Laurence"]
)]
fn find_with_regex_in_table_keeps_row_if_one_column_matches(
    #[case] find_args: &str,
    #[case] expected: impl IntoValue,
) -> Result {
    let code =
        format!("[[name nickname]; [Maurice moe] [Laurence larry]] | find {find_args} | get name");
    test().run(code).expect_value_eq(expected)
}

#[rstest]
#[case::highlighted(
    "--regex moe --invert",
    ["Laurence"]
)]
#[case::plain(
    "--no-highlight --regex moe --invert",
    ["Laurence"]
)]
fn inverted_find_with_regex_in_table_keeps_row_if_none_of_the_columns_matches(
    #[case] find_args: &str,
    #[case] expected: impl IntoValue,
) -> Result {
    let code =
        format!("[[name nickname]; [Maurice moe] [Laurence larry]] | find {find_args} | get name");
    test().run(code).expect_value_eq(expected)
}

#[rstest]
#[case::highlighted(
    "r --columns [nickname]",
    ["Laurence"]
)]
#[case::plain(
    "r --no-highlight --columns [nickname]",
    ["Laurence"]
)]
fn find_in_table_only_keep_rows_with_matches_on_selected_columns(
    #[case] find_args: &str,
    #[case] expected: impl IntoValue,
) -> Result {
    let code =
        format!("[[name nickname]; [Maurice moe] [Laurence larry]] | find {find_args} | get name");
    test().run(code).expect_value_eq(expected)
}

#[rstest]
#[case::highlighted(
    "r --columns [nickname] --invert",
    ["Maurice"]
)]
#[case::plain(
    "r --no-highlight --columns [nickname] --invert",
    ["Maurice"]
)]
fn inverted_find_in_table_keeps_row_if_none_of_the_selected_columns_matches(
    #[case] find_args: &str,
    #[case] expected: impl IntoValue,
) -> Result {
    let code =
        format!("[[name nickname]; [Maurice moe] [Laurence larry]] | find {find_args} | get name");
    test().run(code).expect_value_eq(expected)
}

#[rstest]
#[case::highlighted(
    "Maurice",
    test_table![
        ["name", "nickname", "Age"];
        ["\u{1b}[39m\u{1b}[0m\u{1b}[41;39mMaurice\u{1b}[0m\u{1b}[39m\u{1b}[0m", "moe", 23],
    ]
)]
#[case::plain(
    "--no-highlight Maurice",
    test_table![
        ["name", "nickname", "Age"];
        ["Maurice", "moe", 23],
    ]
)]
fn find_in_table_keeps_row_with_single_matched_and_keeps_other_columns(
    #[case] find_args: &str,
    #[case] expected: impl IntoValue,
) -> Result {
    let code = format!(
        "[[name nickname Age]; [Maurice moe 23] [Laurence larry 67] [William will 18]] | find {find_args}"
    );
    test().run(code).expect_value_eq(expected)
}

#[rstest]
#[case::highlighted(
    "moe William",
    test_table![
        ["name", "nickname", "Age"];
        ["Maurice", "\u{1b}[39m\u{1b}[0m\u{1b}[41;39mmoe\u{1b}[0m\u{1b}[39m\u{1b}[0m", 23],
        ["\u{1b}[39m\u{1b}[0m\u{1b}[41;39mWilliam\u{1b}[0m\u{1b}[39m\u{1b}[0m", "will", 18],
        ["\u{1b}[39m\u{1b}[0m\u{1b}[41;39mWilliam\u{1b}[0m\u{1b}[39m\u{1b}[0m", "bill", 60],
    ]
)]
#[case::plain(
    "--no-highlight moe William",
    test_table![
        ["name", "nickname", "Age"];
        ["Maurice", "moe", 23],
        ["William", "will", 18],
        ["William", "bill", 60],
    ]
)]
fn find_in_table_keeps_row_with_multiple_matched_and_keeps_other_columns(
    #[case] find_args: &str,
    #[case] expected: impl IntoValue,
) -> Result {
    let code = format!(
        "[[name nickname Age]; [Maurice moe 23] [Laurence larry 67] [William will 18] [William bill 60]] | find {find_args}"
    );
    test().run(code).expect_value_eq(expected)
}

#[rstest]
#[case::highlighted(
    "'?'",
    test_table![
        ["d"];
        ["\u{1b}[39ma\u{1b}[0m\u{1b}[41;39m?\u{1b}[0m\u{1b}[39mb\u{1b}[0m"],
    ]
)]
#[case::plain(
    "--no-highlight '?'",
    test_table![["d"]; ["a?b"]]
)]
fn find_with_string_search_with_special_char_1(
    #[case] find_args: &str,
    #[case] expected: impl IntoValue,
) -> Result {
    let code = format!("[[d]; [a?b] [a*b] [a{{1}}b] ] | find {find_args}");
    test().run(code).expect_value_eq(expected)
}

#[rstest]
#[case::highlighted(
    "'*'",
    test_table![
        ["d"];
        ["\u{1b}[39ma\u{1b}[0m\u{1b}[41;39m*\u{1b}[0m\u{1b}[39mb\u{1b}[0m"],
    ]
)]
#[case::plain(
    "--no-highlight '*'",
    test_table![["d"]; ["a*b"]]
)]
fn find_with_string_search_with_special_char_2(
    #[case] find_args: &str,
    #[case] expected: impl IntoValue,
) -> Result {
    let code = format!("[[d]; [a?b] [a*b] [a{{1}}b]] | find {find_args}");
    test().run(code).expect_value_eq(expected)
}

#[rstest]
#[case::highlighted(
    "'{1}'",
    test_table![
        ["d"];
        ["\u{1b}[39ma\u{1b}[0m\u{1b}[41;39m{1}\u{1b}[0m\u{1b}[39mb\u{1b}[0m"],
    ]
)]
#[case::plain(
    "--no-highlight '{1}'",
    test_table![["d"]; ["a{1}b"]]
)]
fn find_with_string_search_with_special_char_3(
    #[case] find_args: &str,
    #[case] expected: impl IntoValue,
) -> Result {
    let code = format!("[[d]; [a?b] [a*b] [a{{1}}b] ] | find {find_args}");
    test().run(code).expect_value_eq(expected)
}

#[rstest]
#[case::highlighted(
    "'['",
    test_table![
        ["d"];
        ["\u{1b}[39ma\u{1b}[0m\u{1b}[41;39m[\u{1b}[0m\u{1b}[39m]b\u{1b}[0m"],
    ]
)]
#[case::plain(
    "--no-highlight '['",
    test_table![["d"]; ["a[]b"]]
)]
fn find_with_string_search_with_special_char_4(
    #[case] find_args: &str,
    #[case] expected: impl IntoValue,
) -> Result {
    let code = format!("[{{d: a?b}} {{d: a*b}} {{d: a{{1}}b}} {{d: a[]b}}] | find {find_args}");
    test().run(code).expect_value_eq(expected)
}

#[rstest]
#[case::highlighted(
    "']'",
    test_table![
        ["d"];
        ["\u{1b}[39ma[\u{1b}[0m\u{1b}[41;39m]\u{1b}[0m\u{1b}[39mb\u{1b}[0m"],
    ]
)]
#[case::plain(
    "--no-highlight ']'",
    test_table![["d"]; ["a[]b"]]
)]
fn find_with_string_search_with_special_char_5(
    #[case] find_args: &str,
    #[case] expected: impl IntoValue,
) -> Result {
    let code = format!("[{{d: a?b}} {{d: a*b}} {{d: a{{1}}b}} {{d: a[]b}}] | find {find_args}");
    test().run(code).expect_value_eq(expected)
}

#[rstest]
#[case::highlighted(
    "'[]'",
    test_table![
        ["d"];
        ["\u{1b}[39ma\u{1b}[0m\u{1b}[41;39m[]\u{1b}[0m\u{1b}[39mb\u{1b}[0m"],
    ]
)]
#[case::plain(
    "--no-highlight '[]'",
    test_table![["d"]; ["a[]b"]]
)]
fn find_with_string_search_with_special_char_6(
    #[case] find_args: &str,
    #[case] expected: impl IntoValue,
) -> Result {
    let code = format!("[{{d: a?b}} {{d: a*b}} {{d: a{{1}}b}} {{d: a[]b}}] | find {find_args}");
    test().run(code).expect_value_eq(expected)
}

#[test]
fn find_in_nested_list_dont_match_bracket() -> Result {
    test()
        .run(r#"[ [foo bar] [foo baz] ] | find "[""#)
        .expect_value_eq(Vec::<Value>::new())
}

#[test]
fn find_and_highlight_in_nested_list() -> Result {
    test()
        .run(r#"[ [foo bar] [foo baz] ] | find "foo""#)
        .expect_value_eq(test_value!([
            [
                "\u{1b}[39m\u{1b}[0m\u{1b}[41;39mfoo\u{1b}[0m\u{1b}[39m\u{1b}[0m",
                "bar"
            ],
            [
                "\u{1b}[39m\u{1b}[0m\u{1b}[41;39mfoo\u{1b}[0m\u{1b}[39m\u{1b}[0m",
                "baz"
            ],
        ]))
}
