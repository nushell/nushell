use crate::tests::{fail_test, run_test, TestResult};

#[test]
fn build_string1() -> TestResult {
    run_test("build-string 'nu' 'shell'", "nushell")
}

#[test]
fn build_string2() -> TestResult {
    run_test("'nu' | each { |it| build-string $it 'shell'}", "nushell")
}

#[test]
fn build_string3() -> TestResult {
    run_test(
        "build-string 'nu' 'shell' | each { |it| build-string $it ' rocks'}",
        "nushell rocks",
    )
}

#[test]
fn build_string4() -> TestResult {
    run_test(
        "['sam','rick','pete'] | each { |it| build-string $it ' is studying'} | get 2",
        "pete is studying",
    )
}

#[test]
fn build_string5() -> TestResult {
    run_test(
        "['sam','rick','pete'] | each { |x| build-string $x ' is studying'} | get 1",
        "rick is studying",
    )
}

#[test]
fn cjk_in_substrings() -> TestResult {
    run_test(
        r#"let s = '[Rust 程序设计语言](title-page.md)'; let start = ($s | str index-of '('); let end = ($s | str index-of ')'); echo ($s | str substring $"($start + 1),($end)")"#,
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
    fail_test(r#"42 in 'abc'"#, "mismatched for operation")
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
        r#"[{"version": "four","package": "abc"},{"version": "three","package": "abc"},{"version": "two","package": "Abc"}]"#,
    )
}
