use crate::tests::{fail_test, run_test, TestResult};

#[test]
fn cjk_in_substrings() -> TestResult {
    run_test(
        r#"let s = '[Rust 程序设计语言](title-page.md)'; let start = ($s | str index-of '('); let end = ($s | str index-of ')'); $s | str substring ($start + 1)..($end)"#,
        "title-page.md",
    )
}

#[test]
fn string_not_in_string() -> TestResult {
    run_test(r#"'d' not-in 'abc'"#, "true")
}

#[test]
fn string_in_string() -> TestResult {
    run_test(r#"'z' in 'abc'"#, "false")
}

#[test]
fn non_string_in_string() -> TestResult {
    fail_test(r#"42 in 'abc'"#, "is not supported")
}

#[test]
fn string_in_record() -> TestResult {
    run_test(r#""a" in ('{ "a": 13, "b": 14 }' | from json)"#, "true")
}

#[test]
fn non_string_in_record() -> TestResult {
    fail_test(
        r#"4 in ('{ "a": 13, "b": 14 }' | from json)"#,
        "mismatch during operation",
    )
}

#[test]
fn string_in_valuestream() -> TestResult {
    run_test(
        r#"
    'Hello' in ("Hello
    World" | lines)"#,
        "true",
    )
}

#[test]
fn single_tick_interpolation() -> TestResult {
    run_test(r#"$'(3 + 4)'"#, "7")
}

#[test]
fn detect_newlines() -> TestResult {
    run_test("'hello\r\nworld' | lines | get 0 | str length", "5")
}

#[test]
fn case_insensitive_sort() -> TestResult {
    run_test(
        r#"[a, B, d, C, f] | sort -i | to json --raw"#,
        "[\"a\",\"B\",\"C\",\"d\",\"f\"]",
    )
}

#[test]
fn case_insensitive_sort_columns() -> TestResult {
    run_test(
        r#"[[version, package]; ["two", "Abc"], ["three", "abc"], ["four", "abc"]] | sort-by -i package version | to json --raw"#,
        r#"[{"version":"four","package":"abc"},{"version":"three","package":"abc"},{"version":"two","package":"Abc"}]"#,
    )
}
