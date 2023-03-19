use crate::tests::{fail_test, run_test, TestResult};

#[test]
fn list_annotations() -> TestResult {
    let input = "def run [list: list<int>] {$list | length}; run [2 5 4]";
    let expected = "3";
    run_test(input, expected)
}

#[test]
fn list_annotations_empty_1() -> TestResult {
    let input = "def run [list: list] {$list | length}; run [2 5 4]";
    let expected = "3";
    run_test(input, expected)
}

#[test]
fn list_annotations_empty_2() -> TestResult {
    let input = "def run [list: list<>] {$list | length}; run [2 5 4]";
    let expected = "3";
    run_test(input, expected)
}

#[test]
fn list_annotations_empty_3() -> TestResult {
    let input = "def run [list: list< >] {$list | length}; run [2 5 4]";
    let expected = "3";
    run_test(input, expected)
}

#[test]
fn list_annotations_empty_4() -> TestResult {
    let input = "def run [list: list<\n>] {$list | length}; run [2 5 4]";
    let expected = "3";
    run_test(input, expected)
}

#[test]
fn list_annotations_nested() -> TestResult {
    let input = "def run [list: list<list<float>>] {$list | length}; run [ [2.0] [5.0] [4.0]]";
    let expected = "3";
    run_test(input, expected)
}

#[test]
fn list_annotations_unknown_inner_type() -> TestResult {
    let input = "def run [list: list<str>] {$list | length}; run ['nushell' 'nunu' 'nana']";
    let expected = "unknown type";
    fail_test(input, expected)
}

#[test]
fn list_annotations_nested_unknown_inner() -> TestResult {
    let input = "def run [list: list<list<str>>] {$list | length}; run [ [nushell] [nunu] [nana]]";
    let expected = "unknown type";
    fail_test(input, expected)
}

#[test]
fn list_annotations_unterminated() -> TestResult {
    let input = "def run [list: list<string] {$list | length}; run [nu she ll]";
    let expected = "expected closing >";
    fail_test(input, expected)
}

#[test]
fn list_annotations_nested_unterminated() -> TestResult {
    let input = "def run [list: list<list<>] {$list | length}; run [2 5 4]";
    let expected = "expected closing >";
    fail_test(input, expected)
}

#[test]
fn list_annotations_space_within_1() -> TestResult {
    let input = "def run [list: list< range>] {$list | length}; run [2..32 5..<64 4..128]";
    let expected = "3";
    run_test(input, expected)
}

#[test]
fn list_annotations_space_within_2() -> TestResult {
    let input = "def run [list: list<number >] {$list | length}; run [2 5 4]";
    let expected = "3";
    run_test(input, expected)
}

#[test]
fn list_annotations_space_within_3() -> TestResult {
    let input = "def run [list: list< int >] {$list | length}; run [2 5 4]";
    let expected = "3";
    run_test(input, expected)
}

#[test]
fn list_annotations_space_before() -> TestResult {
    let input = "def run [list: list <int>] {$list | length}; run [2 5 4]";
    let expected = "expected valid variable name for this parameter";
    fail_test(input, expected)
}

#[test]
fn list_annotations_unknown_separators() -> TestResult {
    let input = "def run [list: list<int, string>] {$list | length}; run [2 5 4]";
    let expected = "unknown type";
    fail_test(input, expected)
}
