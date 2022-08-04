use crate::tests::{fail_test, run_test, TestResult};

#[test]
fn contains() -> TestResult {
    run_test(r#"'foobarbaz' =~ bar"#, "true")
}

#[test]
fn contains_case_insensitive() -> TestResult {
    run_test(r#"'foobarbaz' =~ '(?i)BaR'"#, "true")
}

#[test]
fn not_contains() -> TestResult {
    run_test(r#"'foobarbaz' !~ asdf"#, "true")
}

#[test]
fn match_full_line() -> TestResult {
    run_test(r#"'foobarbaz' =~ '^foobarbaz$'"#, "true")
}

#[test]
fn not_match_full_line() -> TestResult {
    run_test(r#"'foobarbaz' !~ '^foobarbaz$'"#, "false")
}

#[test]
fn starts_with() -> TestResult {
    run_test(r#"'foobarbaz' =~ '^foo'"#, "true")
}

#[test]
fn not_starts_with() -> TestResult {
    run_test(r#"'foobarbaz' !~ '^foo'"#, "false")
}

#[test]
fn ends_with() -> TestResult {
    run_test(r#"'foobarbaz' =~ 'baz$'"#, "true")
}

#[test]
fn not_ends_with() -> TestResult {
    run_test(r#"'foobarbaz' !~ 'baz$'"#, "false")
}

#[test]
fn where_works() -> TestResult {
    run_test(
        r#"[{name: somefile.txt} {name: anotherfile.csv }] | where name =~ ^s | get name.0"#,
        "somefile.txt",
    )
}

#[test]
fn where_not_works() -> TestResult {
    run_test(
        r#"[{name: somefile.txt} {name: anotherfile.csv }] | where name !~ ^s | get name.0"#,
        "anotherfile.csv",
    )
}

#[test]
fn invalid_regex_fails() -> TestResult {
    fail_test(r#"'foo' =~ '['"#, "Invalid character class")
}

#[test]
fn invalid_not_regex_fails() -> TestResult {
    fail_test(r#"'foo' !~ '['"#, "Invalid character class")
}

#[test]
fn regex_on_int_fails() -> TestResult {
    fail_test(r#"33 =~ foo"#, "Types mismatched")
}

#[test]
fn not_regex_on_int_fails() -> TestResult {
    fail_test(r#"33 !~ foo"#, "Types mismatched")
}
