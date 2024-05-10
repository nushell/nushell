use crate::repl::tests::{fail_test, run_test, TestResult};

#[test]
fn list_annotations() -> TestResult {
    let input = "def run [list: list<int>] {$list | length}; run [2 5 4]";
    let expected = "3";
    run_test(input, expected)
}

#[test]
fn list_annotations_unknown_prefix() -> TestResult {
    let input = "def run [list: listint>] {$list | length}; run [2 5 4]";
    let expected = "unknown type";
    fail_test(input, expected)
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

#[test]
fn list_annotations_with_default_val_1() -> TestResult {
    let input = "def run [list: list<int> = [2 5 4]] {$list | length}; run";
    let expected = "3";
    run_test(input, expected)
}

#[test]
fn list_annotations_with_default_val_2() -> TestResult {
    let input = "def run [list: list<string> = [2 5 4]] {$list | length}; run";
    let expected = "Default value wrong type";
    fail_test(input, expected)
}

#[test]
fn list_annotations_with_extra_characters() -> TestResult {
    let input = "def run [list: list<int>extra] {$list | length}; run [1 2 3]";
    let expected = "Extra characters in the parameter name";
    fail_test(input, expected)
}

#[test]
fn record_annotations_none() -> TestResult {
    let input = "def run [rec: record] { $rec }; run {} | describe";
    let expected = "record";
    run_test(input, expected)
}

#[test]
fn record_annotations() -> TestResult {
    let input = "def run [rec: record<age: int>] { $rec }; run {age: 3} | describe";
    let expected = "record<age: int>";
    run_test(input, expected)
}

#[test]
fn record_annotations_two_types() -> TestResult {
    let input = "def run [rec: record<name: string age: int>] { $rec }; run {name: nushell age: 3} | describe";
    let expected = "record<name: string, age: int>";
    run_test(input, expected)
}

#[test]
fn record_annotations_two_types_comma_sep() -> TestResult {
    let input = "def run [rec: record<name: string, age: int>] { $rec }; run {name: nushell age: 3} | describe";
    let expected = "record<name: string, age: int>";
    run_test(input, expected)
}

#[test]
fn record_annotations_key_with_no_type() -> TestResult {
    let input = "def run [rec: record<name>] { $rec }; run {name: nushell} | describe";
    let expected = "record<name: string>";
    run_test(input, expected)
}

#[test]
fn record_annotations_two_types_one_with_no_type() -> TestResult {
    let input =
        "def run [rec: record<name: string, age>] { $rec }; run {name: nushell age: 3} | describe";
    let expected = "record<name: string, age: int>";
    run_test(input, expected)
}

#[test]
fn record_annotations_two_types_both_with_no_types() -> TestResult {
    let input = "def run [rec: record<name age>] { $rec }; run {name: nushell age: 3} | describe";
    let expected = "record<name: string, age: int>";
    run_test(input, expected)
}

#[test]
fn record_annotations_nested() -> TestResult {
    let input = "def run [
        err: record<
            msg: string,
            label: record<
               text: string
               start: int,
               end: int,
            >>
    ] {
        $err 
    }; run {
        msg: 'error message'
        label: {
            text: 'here is the error'
            start: 0
            end: 69
        }
    } | describe";
    let expected = "record<msg: string, label: record<text: string, start: int, end: int>>";
    run_test(input, expected)
}

#[test]
fn record_annotations_type_inference_1() -> TestResult {
    let input = "def run [rec: record<age: any>] { $rec }; run {age: 2wk} | describe";
    let expected = "record<age: duration>";
    run_test(input, expected)
}

#[test]
fn record_annotations_type_inference_2() -> TestResult {
    let input = "def run [rec: record<size>] { $rec }; run {size: 2mb} | describe";
    let expected = "record<size: filesize>";
    run_test(input, expected)
}

#[test]
fn record_annotations_not_terminated() -> TestResult {
    let input = "def run [rec: record<age: int] { $rec }";
    let expected = "expected closing >";
    fail_test(input, expected)
}

#[test]
fn record_annotations_not_terminated_inner() -> TestResult {
    let input = "def run [rec: record<name: string, repos: list<string>] { $rec }";
    let expected = "expected closing >";
    fail_test(input, expected)
}

#[test]
fn record_annotations_no_type_after_colon() -> TestResult {
    let input = "def run [rec: record<name: >] { $rec }";
    let expected = "type after colon";
    fail_test(input, expected)
}

#[test]
fn record_annotations_type_mismatch_key() -> TestResult {
    let input = "def run [rec: record<name: string>] { $rec }; run {nme: nushell}";
    let expected = "expected record<name: string>, found record<nme: string>";
    fail_test(input, expected)
}

#[test]
fn record_annotations_type_mismatch_shape() -> TestResult {
    let input = "def run [rec: record<age: int>] { $rec }; run {age: 2wk}";
    let expected = "expected record<age: int>, found record<age: duration>";
    fail_test(input, expected)
}

#[test]
fn record_annotations_with_extra_characters() -> TestResult {
    let input = "def run [list: record<int>extra] {$list | length}; run [1 2 3]";
    let expected = "Extra characters in the parameter name";
    fail_test(input, expected)
}

#[test]
fn table_annotations_none() -> TestResult {
    let input = "def run [t: table] { $t }; run [[]; []] | describe";
    let expected = "table";
    run_test(input, expected)
}

#[test]
fn table_annotations() -> TestResult {
    let input = "def run [t: table<age: int>] { $t }; run [[age]; [3]] | describe";
    let expected = "table<age: int>";
    run_test(input, expected)
}

#[test]
fn table_annotations_two_types() -> TestResult {
    let input = "\
def run [t: table<name: string age: int>] { $t };
run [[name, age]; [nushell, 3]] | describe";
    let expected = "table<name: string, age: int>";
    run_test(input, expected)
}

#[test]
fn table_annotations_two_types_comma_sep() -> TestResult {
    let input = "\
def run [t: table<name: string, age: int>] { $t };
run [[name, age]; [nushell, 3]] | describe";
    let expected = "table<name: string, age: int>";
    run_test(input, expected)
}

#[test]
fn table_annotations_key_with_no_type() -> TestResult {
    let input = "def run [t: table<name>] { $t }; run [[name]; [nushell]] | describe";
    let expected = "table<name: string>";
    run_test(input, expected)
}

#[test]
fn table_annotations_two_types_one_with_no_type() -> TestResult {
    let input = "\
def run [t: table<name: string, age>] { $t };
run [[name, age]; [nushell, 3]] | describe";
    let expected = "table<name: string, age: int>";
    run_test(input, expected)
}

#[test]
fn table_annotations_two_types_both_with_no_types() -> TestResult {
    let input = "\
def run [t: table<name, age>] { $t };
run [[name, age]; [nushell, 3]] | describe";
    let expected = "table<name: string, age: int>";
    run_test(input, expected)
}

#[test]
fn table_annotations_type_inference_1() -> TestResult {
    let input = "def run [t: table<age: any>] { $t }; run [[age]; [2wk]] | describe";
    let expected = "table<age: duration>";
    run_test(input, expected)
}

#[test]
fn table_annotations_type_inference_2() -> TestResult {
    let input = "def run [t: table<size>] { $t }; run [[size]; [2mb]] | describe";
    let expected = "table<size: filesize>";
    run_test(input, expected)
}

#[test]
fn table_annotations_not_terminated() -> TestResult {
    let input = "def run [t: table<age: int] { $t }";
    let expected = "expected closing >";
    fail_test(input, expected)
}

#[test]
fn table_annotations_not_terminated_inner() -> TestResult {
    let input = "def run [t: table<name: string, repos: list<string>] { $t }";
    let expected = "expected closing >";
    fail_test(input, expected)
}

#[test]
fn table_annotations_no_type_after_colon() -> TestResult {
    let input = "def run [t: table<name: >] { $t }";
    let expected = "type after colon";
    fail_test(input, expected)
}

#[test]
fn table_annotations_type_mismatch_column() -> TestResult {
    let input = "def run [t: table<name: string>] { $t }; run [[nme]; [nushell]]";
    let expected = "expected table<name: string>, found table<nme: string>";
    fail_test(input, expected)
}

#[test]
fn table_annotations_type_mismatch_shape() -> TestResult {
    let input = "def run [t: table<age: int>] { $t }; run [[age]; [2wk]]";
    let expected = "expected table<age: int>, found table<age: duration>";
    fail_test(input, expected)
}

#[test]
fn table_annotations_with_extra_characters() -> TestResult {
    let input = "def run [t: table<int>extra] {$t | length}; run [[int]; [8]]";
    let expected = "Extra characters in the parameter name";
    fail_test(input, expected)
}
