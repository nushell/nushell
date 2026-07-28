use indoc::indoc;
use nu_test_support::prelude::*;
use rstest::rstest;

#[test]
fn detect_columns_with_legacy() -> Result {
    test()
        .run_with_data("$in | detect columns", "c1 c2 c3 c4 c5\na b c d e")
        .expect_value_eq(test_table![
            ["c1", "c2", "c3", "c4", "c5"];
            ["a", "b", "c", "d", "e"]
        ])
}

#[rstest]
#[case::start("0..1", test_table![
    ["c1", "c3", "c4", "c5"];
    ["a b", "c", "d", "e"]
])]
#[case::negative("(-2)..(-1)", test_table![
    ["c1", "c2", "c3", "c4"];
    ["a", "b", "c", "d e"]
])]
#[case::open_ended("2..", test_table![
    ["c1", "c2", "c3"];
    ["a", "b", "c d e"]
])]
fn detect_columns_with_legacy_and_flag_c(
    #[case] range: &str,
    #[case] expected: impl IntoValue,
) -> Result {
    test()
        .run_with_data(
            format!("$in | detect columns --combine-columns {range}"),
            "c1 c2 c3 c4 c5\na b c d e",
        )
        .expect_value_eq(expected)
}

#[test]
fn detect_columns_with_flag_c() -> Result {
    let input = indoc! {"
        total 284K
        drwxr-xr-x  2 root root 4.0K Mar 20 08:28 =
        drwxr-xr-x  4 root root 4.0K Mar 20 08:18 ~
        -rw-r--r--  1 root root 3.0K Mar 20 07:23 ~asdf
    "};

    test()
        .run_with_data("$in | detect columns -c 5..6 -s 1 --no-headers", input)
        .expect_value_eq(test_table![
            ["column0", "column1", "column2", "column3", "column4", "column5", "column7", "column8"];
            ["drwxr-xr-x", "2", "root", "root", "4.0K", "Mar 20", "08:28", "="],
            ["drwxr-xr-x", "4", "root", "root", "4.0K", "Mar 20", "08:18", "~"],
            ["-rw-r--r--", "1", "root", "root", "3.0K", "Mar 20", "07:23", "~asdf"]
        ])
}

#[test]
fn detect_columns_may_fail() -> Result {
    // Test case where column detection produces duplicate column names.
    // With our updated implementation, when detection fails due to mismatched
    // columns, data goes to "data" column instead of throwing an error.
    // But duplicate column headers still cause an error.
    test()
        .run_with_data(
            r#"try { $in | detect columns } catch { "failed" }"#,
            "cat cat\nkitty woof",
        )
        .expect_value_eq("failed")
}

// Test with iptab-like output containing box drawing characters.
// When column detection fails, all rows should be output in a consistent
// "data" column, preserving the original content including box characters.
#[rstest]
// All rows should be in the "data" column when detection fails (6 lines total).
#[case::all_rows_are_data("$in | detect columns | get data | length", 6)]
// The "data" column should contain the full original text, including 'addrs'.
#[case::preserves_header_text(
    r#"$in | detect columns | get data | any {|line| $line | str contains "addrs"}"#,
    true
)]
// Verify the box-only lines still have box characters (+ and -).
#[case::preserves_box_chars(
    r#"$in | detect columns | get data | any {|line| ($line | str contains "+") and ($line | str contains "-")}"#,
    true
)]
// Verify data lines preserve the pipe | characters.
#[case::preserves_pipes(
    r#"$in | detect columns | get data | where {|line| $line | str contains "addrs"} | first | str contains "|""#,
    true
)]
fn detect_columns_preserves_original_content_on_mismatch(
    #[case] code: &str,
    #[case] expected: impl IntoValue,
) -> Result {
    let input = indoc! {"
        +----------------------------------------------+
        | addrs   bits   pref   class  mask            |
        +----------------------------------------------+
        |     1      0    /32          255.255.255.255 |
        |     2      1    /31          255.255.255.254 |
        +----------------------------------------------+
    "};

    test().run_with_data(code, input).expect_value_eq(expected)
}

// When --ignore-box-chars is used, lines consisting entirely of box drawing
// characters should be ignored.
#[rstest]
// Without flag: separator line causes column mismatch, so all rows go to "data"
// (including the first line which is used as header attempt). Header is
// "col1 col2 col3" (3 cols), separator is "----+----+----" (1 col), so all 3
// rows go to "data" when the first data row does not match the header.
#[case::without_flag("$in | detect columns | get data? | length", 3)]
// With --ignore-box-chars flag: the separator line is ignored.
#[case::with_flag("$in | detect columns --ignore-box-chars | get col1 | first", "val1")]
fn detect_columns_ignore_box_chars_flag(
    #[case] code: &str,
    #[case] expected: impl IntoValue,
) -> Result {
    let input = indoc! {"
        col1 col2 col3
        ----+----+----
        val1 val2 val3"};

    test().run_with_data(code, input).expect_value_eq(expected)
}

// Regression test for the panic reported when a data row contained a
// multibyte character (... ellipsis) and a header offset landed inside it.
// The command should complete successfully and provide the expected field.
#[test]
fn detect_columns_no_panic_with_multibyte_data() -> Result {
    let input = indoc! {"
        Name                   Id                                 Version      Match     Source
        katharsis              arghena.katharsis                  1.0.0-canar… Tag: rust winget
    "};

    test()
        .run_with_data(
            "$in | detect columns --ignore-box-chars | get Version | first",
            input,
        )
        .expect_value_eq("1.0.0-canar…")
}
