use crate::tests::{fail_test, run_test, TestResult};

#[test]
fn list_annotations() -> TestResult {
    let input = "def run [list: [int]] {$list | length}; run [2 5 4]";
    let expected = "3";
    run_test(input, expected)
}

#[test]
fn list_annotations_unknown_prefix() -> TestResult {
    let input = "def run [list: int]] {$list | length}; run [2 5 4]";
    let expected = "unknown type";
    fail_test(input, expected)
}

#[test]
fn list_annotations_empty_1() -> TestResult {
    let input = "def run [list: []] {$list | length}; run [2 5 4]";
    let expected = "3";
    run_test(input, expected)
}

#[test]
fn list_annotations_empty_2() -> TestResult {
    let input = "def run [list: []] {$list | length}; run [2 5 4]";
    let expected = "3";
    run_test(input, expected)
}

#[test]
fn list_annotations_empty_3() -> TestResult {
    let input = "def run [list: [ ]] {$list | length}; run [2 5 4]";
    let expected = "3";
    run_test(input, expected)
}

#[test]
fn list_annotations_empty_4() -> TestResult {
    let input = "def run [list: [\n]] {$list | length}; run [2 5 4]";
    let expected = "3";
    run_test(input, expected)
}

#[test]
fn list_annotations_nested() -> TestResult {
    let input = "def run [list: [[float]]] {$list | length}; run [ [2.0] [5.0] [4.0]]";
    let expected = "3";
    run_test(input, expected)
}

#[test]
fn list_annotations_unknown_inner_type() -> TestResult {
    let input = "def run [list: [str]] {$list | length}; run ['nushell' 'nunu' 'nana']";
    let expected = "unknown type";
    fail_test(input, expected)
}

#[test]
fn list_annotations_nested_unknown_inner() -> TestResult {
    let input = "def run [list: [[str]]] {$list | length}; run [ [nushell] [nunu] [nana]]";
    let expected = "unknown type";
    fail_test(input, expected)
}

#[test]
fn list_annotations_unterminated() -> TestResult {
    let input = "def run [list: [string] {$list | length}; run [nu she ll]";
    let expected = "eof";
    fail_test(input, expected)
}

#[test]
fn list_annotations_nested_unterminated() -> TestResult {
    let input = "def run [list: [[]] {$list | length}; run [2 5 4]";
    let expected = "eof";
    fail_test(input, expected)
}

#[test]
fn list_annotations_space_within_1() -> TestResult {
    let input = "def run [list: [ range]] {$list | length}; run [2..32 5..<64 4..128]";
    let expected = "3";
    run_test(input, expected)
}

#[test]
fn list_annotations_space_within_2() -> TestResult {
    let input = "def run [list: [number ]] {$list | length}; run [2 5 4]";
    let expected = "3";
    run_test(input, expected)
}

#[test]
fn list_annotations_space_within_3() -> TestResult {
    let input = "def run [list: [ int ]] {$list | length}; run [2 5 4]";
    let expected = "3";
    run_test(input, expected)
}

#[test]
fn list_annotations_unknown_separators() -> TestResult {
    let input = "def run [list: [int, string]] {$list | length}; run [2 5 4]";
    let expected = "unknown type";
    fail_test(input, expected)
}

#[test]
fn list_annotations_with_default_val_1() -> TestResult {
    let input = "def run [list: [int] = [2 5 4]] {$list | length}; run";
    let expected = "3";
    run_test(input, expected)
}

#[test]
fn list_annotations_with_default_val_2() -> TestResult {
    let input = "def run [list: [string] = [2 5 4]] {$list | length}; run";
    let expected = "Default value wrong type";
    fail_test(input, expected)
}

#[test]
fn list_annotations_with_extra_characters() -> TestResult {
    let input = "def run [list: [int]extra] {$list | length}; run [1 2 3]";
    let expected = "unknown type";
    fail_test(input, expected)
}
