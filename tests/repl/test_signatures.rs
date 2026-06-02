use nu_protocol::Type;
use nu_test_support::prelude::*;

use crate::repl::tests::{TestResult, fail_test, run_test};
use rstest::rstest;

#[test]
fn list_annotations() -> TestResult {
    let input = "def my_run [list: list<int>] {$list | length}; my_run [2 5 4]";
    let expected = "3";
    run_test(input, expected)
}

#[test]
fn list_annotations_unknown_prefix() -> TestResult {
    let input = "def my_run [list: listint>] {$list | length}; my_run [2 5 4]";
    let expected = "unknown type";
    fail_test(input, expected)
}

#[test]
fn list_annotations_empty_1() -> TestResult {
    let input = "def my_run [list: list] {$list | length}; my_run [2 5 4]";
    let expected = "3";
    run_test(input, expected)
}

#[test]
fn list_annotations_empty_2() -> TestResult {
    let input = "def my_run [list: list<>] {$list | length}; my_run [2 5 4]";
    let expected = "3";
    run_test(input, expected)
}

#[test]
fn list_annotations_empty_3() -> TestResult {
    let input = "def my_run [list: list< >] {$list | length}; my_run [2 5 4]";
    let expected = "3";
    run_test(input, expected)
}

#[test]
fn list_annotations_empty_4() -> TestResult {
    let input = "def my_run [list: list<\n>] {$list | length}; my_run [2 5 4]";
    let expected = "3";
    run_test(input, expected)
}

#[test]
fn list_annotations_nested() -> TestResult {
    let input =
        "def my_run [list: list<list<float>>] {$list | length}; my_run [ [2.0] [5.0] [4.0]]";
    let expected = "3";
    run_test(input, expected)
}

#[test]
fn list_annotations_unknown_inner_type() -> TestResult {
    let input = "def my_run [list: list<str>] {$list | length}; my_run ['nushell' 'nunu' 'nana']";
    let expected = "unknown type";
    fail_test(input, expected)
}

#[test]
fn list_annotations_nested_unknown_inner() -> TestResult {
    let input =
        "def my_run [list: list<list<str>>] {$list | length}; my_run [ [nushell] [nunu] [nana]]";
    let expected = "unknown type";
    fail_test(input, expected)
}

#[test]
fn list_annotations_unterminated() -> TestResult {
    let input = "def my_run [list: list<string] {$list | length}; my_run [nu she ll]";
    let expected = "expected closing >";
    fail_test(input, expected)
}

#[test]
fn list_annotations_nested_unterminated() -> TestResult {
    let input = "def my_run [list: list<list<>] {$list | length}; my_run [2 5 4]";
    let expected = "expected closing >";
    fail_test(input, expected)
}

#[test]
fn list_annotations_space_within_1() -> TestResult {
    let input = "def my_run [list: list< range>] {$list | length}; my_run [2..32 5..<64 4..128]";
    let expected = "3";
    run_test(input, expected)
}

#[test]
fn list_annotations_space_within_2() -> TestResult {
    let input = "def my_run [list: list<number >] {$list | length}; my_run [2 5 4]";
    let expected = "3";
    run_test(input, expected)
}

#[test]
fn list_annotations_space_within_3() -> TestResult {
    let input = "def my_run [list: list< int >] {$list | length}; my_run [2 5 4]";
    let expected = "3";
    run_test(input, expected)
}

#[test]
fn list_annotations_space_before() -> TestResult {
    let input = "def my_run [list: list <int>] {$list | length}; my_run [2 5 4]";
    let expected = "expected valid variable name for this parameter";
    fail_test(input, expected)
}

#[test]
fn list_annotations_unknown_separators() -> TestResult {
    let input = "def my_run [list: list<int, string>] {$list | length}; my_run [2 5 4]";
    let expected = "only one parameter allowed";
    fail_test(input, expected)
}

#[test]
fn list_annotations_with_default_val_1() -> TestResult {
    let input = "def my_run [list: list<int> = [2 5 4]] {$list | length}; my_run";
    let expected = "3";
    run_test(input, expected)
}

#[test]
fn list_annotations_with_default_val_2() -> TestResult {
    let input = "def my_run [list: list<string> = [2 5 4]] {$list | length}; my_run";
    let expected = "3";
    run_test(input, expected)
}

#[test]
fn list_annotations_with_extra_characters() -> TestResult {
    let input = "def my_run [list: list<int>extra] {$list | length}; my_run [1 2 3]";
    let expected = "Extra characters in the parameter name";
    fail_test(input, expected)
}

#[test]
fn record_annotations_none() -> TestResult {
    let input = "def my_run [rec: record] { $rec }; my_run {} | describe";
    let expected = "record";
    run_test(input, expected)
}

#[test]
fn record_annotations() -> TestResult {
    let input = "def my_run [rec: record<age: int>] { $rec }; my_run {age: 3} | describe";
    let expected = "record<age: int>";
    run_test(input, expected)
}

#[test]
fn record_annotations_two_types() -> TestResult {
    let input = "def my_run [rec: record<name: string age: int>] { $rec }; my_run {name: nushell age: 3} | describe";
    let expected = "record<name: string, age: int>";
    run_test(input, expected)
}

#[test]
fn record_annotations_two_types_comma_sep() -> TestResult {
    let input = "def my_run [rec: record<name: string, age: int>] { $rec }; my_run {name: nushell age: 3} | describe";
    let expected = "record<name: string, age: int>";
    run_test(input, expected)
}

#[test]
fn record_annotations_key_with_no_type() -> TestResult {
    let input = "def my_run [rec: record<name>] { $rec }; my_run {name: nushell} | describe";
    let expected = "record<name: string>";
    run_test(input, expected)
}

#[test]
fn record_annotations_two_types_one_with_no_type() -> TestResult {
    let input = "def my_run [rec: record<name: string, age>] { $rec }; my_run {name: nushell age: 3} | describe";
    let expected = "record<name: string, age: int>";
    run_test(input, expected)
}

#[test]
fn record_annotations_two_types_both_with_no_types() -> TestResult {
    let input =
        "def my_run [rec: record<name age>] { $rec }; my_run {name: nushell age: 3} | describe";
    let expected = "record<name: string, age: int>";
    run_test(input, expected)
}

#[test]
fn record_annotations_nested() -> TestResult {
    let input = "def my_run [
        err: record<
            msg: string,
            label: record<
               text: string
               start: int,
               end: int,
            >>
    ] {
        $err 
    }; my_run {
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
    let input = "def my_run [rec: record<age: any>] { $rec }; my_run {age: 2wk} | describe";
    let expected = "record<age: duration>";
    run_test(input, expected)
}

#[test]
fn record_annotations_type_inference_2() -> TestResult {
    let input = "def my_run [rec: record<size>] { $rec }; my_run {size: 2mb} | describe";
    let expected = "record<size: filesize>";
    run_test(input, expected)
}

#[test]
fn record_annotations_not_terminated() -> TestResult {
    let input = "def my_run [rec: record<age: int] { $rec }";
    let expected = "expected closing >";
    fail_test(input, expected)
}

#[test]
fn record_annotations_not_terminated_inner() -> TestResult {
    let input = "def my_run [rec: record<name: string, repos: list<string>] { $rec }";
    let expected = "expected closing >";
    fail_test(input, expected)
}

#[test]
fn record_annotations_no_type_after_colon() -> TestResult {
    let input = "def my_run [rec: record<name: >] { $rec }";
    let expected = "type after colon";
    fail_test(input, expected)
}

#[test]
fn record_annotations_type_mismatch_key() -> TestResult {
    let input = "def my_run [rec: record<name: string>] { $rec }; my_run {nme: nushell}";
    let expected = "expected record<name: string>, found record<nme: string>";
    fail_test(input, expected)
}

#[test]
fn record_annotations_type_mismatch_shape() -> TestResult {
    let input = "def my_run [rec: record<age: int>] { $rec }; my_run {age: 2wk}";
    let expected = "expected record<age: int>, found record<age: duration>";
    fail_test(input, expected)
}

#[test]
fn record_annotations_with_extra_characters() -> TestResult {
    let input = "def my_run [list: record<int>extra] {$list | length}; my_run [1 2 3]";
    let expected = "Extra characters in the parameter name";
    fail_test(input, expected)
}

#[test]
fn table_annotations_none() -> TestResult {
    let input = "def my_run [t: table] { $t }; my_run [[]; []] | describe";
    let expected = "table";
    run_test(input, expected)
}

#[rstest]
fn table_annotations(
    #[values(true, false)] list_annotation: bool,
    #[values(
        ("age: int", "age: int", "[[age]; [3]]"  ),
        ("name: string age: int", "name: string, age: int", "[[name, age]; [nushell, 3]]"  ),
        ("name: string, age: int", "name: string, age: int", "[[name, age]; [nushell, 3]]"  ),
        ("name", "name: string", "[[name]; [nushell]]"),
        ("name: string, age", "name: string, age: int", "[[name, age]; [nushell, 3]]"),
        ("name, age", "name: string, age: int", "[[name, age]; [nushell, 3]]"),
        ("age: any", "age: duration", "[[age]; [2wk]]"),
        ("size", "size: filesize", "[[size]; [2mb]]")
    )]
    record_annotation_data: (&str, &str, &str),
) -> TestResult {
    let (record_annotation, inferred_type, data) = record_annotation_data;

    let type_annotation = match list_annotation {
        true => format!("list<record<{record_annotation}>>"),
        false => format!("table<{record_annotation}>"),
    };
    let input = format!("def my_run [t: {type_annotation}] {{ $t }}; my_run {data} | describe");
    let expected = format!("table<{inferred_type}>");
    run_test(&input, &expected)
}

#[test]
fn table_annotations_not_terminated() -> TestResult {
    let input = "def my_run [t: table<age: int] { $t }";
    let expected = "expected closing >";
    fail_test(input, expected)
}

#[test]
fn table_annotations_not_terminated_inner() -> TestResult {
    let input = "def my_run [t: table<name: string, repos: list<string>] { $t }";
    let expected = "expected closing >";
    fail_test(input, expected)
}

#[test]
fn table_annotations_no_type_after_colon() -> TestResult {
    let input = "def my_run [t: table<name: >] { $t }";
    let expected = "type after colon";
    fail_test(input, expected)
}

#[test]
fn table_annotations_type_mismatch_column() -> TestResult {
    let input = "def my_run [t: table<name: string>] { $t }; my_run [[nme]; [nushell]]";
    let expected = "expected table<name: string>, found table<nme: string>";
    fail_test(input, expected)
}

#[test]
fn table_annotations_type_mismatch_shape() -> TestResult {
    let input = "def my_run [t: table<age: int>] { $t }; my_run [[age]; [2wk]]";
    let expected = "expected table<age: int>, found table<age: duration>";
    fail_test(input, expected)
}

#[test]
fn table_annotations_with_extra_characters() -> TestResult {
    let input = "def my_run [t: table<int>extra] {$t | length}; my_run [[int]; [8]]";
    let expected = "Extra characters in the parameter name";
    fail_test(input, expected)
}

#[rstest]
fn oneof_annotations(
    #[values(
        ("cell-path, list<cell-path>", "a.b.c", "cell-path"),
        ("cell-path, list<cell-path>", "[a.b.c d.e.f]", "list<cell-path>"),
        ("closure, any", "{}", "closure"),
        ("closure, any", "{a: 1}", "record<a: int>"),
    )]
    annotation_data: (&str, &str, &str),
) -> TestResult {
    let (types, argument, expected) = annotation_data;

    let input = format!("def my_run [t: oneof<{types}>] {{ $t }}; my_run {argument} | describe");
    run_test(&input, expected)
}

#[rstest]
#[case::table_record("oneof<record, table>", "{item: 1}", Type::Record(vec![("item".to_string(), Type::Int)].into()))]
#[case::table_list_of_record("oneof<record, table>", "[{item: 1}]", Type::Table(vec![("item".to_string(), Type::Int)].into()))]
#[case::table_table("oneof<record, table>", "[[item]; [1]]", Type::Table(vec![("item".to_string(), Type::Int)].into()))]
#[case::closure_cell_path("oneof<cell-path, closure>", "foo", Type::CellPath)]
#[case::closure_closure_with_bars("oneof<cell-path, closure>", "{|| get foo }", Type::Closure)]
#[case::closure_closure_without_bars("oneof<cell-path, closure>", "{ get foo }", Type::Closure)]
#[case::closure_closure_empty_with_bars("oneof<cell-path, closure>", "{|| }", Type::Closure)]
#[ignore = "not fixed yet"]
#[case::closure_closure_empty_without_bars("oneof<cell-path, closure>", "{ }", Type::Closure)]
fn oneof_positional_named(
    #[values(("value", ""), ("--value", "--value"))] (param_definition, arg_prefix): (&str, &str),
    #[case] type_annotation: &str,
    #[case] arg_src: &str,
    #[case] expected_type: Type,
) -> Result {
    let mut tester = test();
    let () = tester.run(format!(
        "def cmd [{param_definition}: {type_annotation}] {{ $value }}"
    ))?;
    let out: Value = tester.run(format!("cmd {arg_prefix} {arg_src}"))?;
    assert_eq!(out.get_type(), expected_type);
    Ok(())
}

#[rstest]
#[case::correct_type_(run_test, "{a: 1}", "")]
#[case::correct_type_(run_test, "{a: null}", "")]
#[case::parse_time_incorrect_type(fail_test, "{a: 1.0}", "parser::type_mismatch")]
#[case::run_time_incorrect_type(fail_test, "(echo {a: 1.0})", "shell::cant_convert")]
fn oneof_type_checking(
    #[case] testfn: fn(&str, &str) -> TestResult,
    #[case] argument: &str,
    #[case] expect: &str,
) {
    let _ = testfn(
        &format!("def my_run [p: record<a: oneof<int, nothing>>] {{ }}; my_run {argument}"),
        expect,
    );
}

#[test]
fn oneof_annotations_not_terminated() -> TestResult {
    let input = "def my_run [t: oneof<binary, string] { $t }";
    let expected = "expected closing >";
    fail_test(input, expected)
}

#[test]
fn oneof_annotations_with_extra_characters() -> TestResult {
    let input = "def my_run [t: oneof<int, string>extra] {$t}";
    let expected = "Extra characters in the parameter name";
    fail_test(input, expected)
}

#[rstest]
#[case("{ |a $a }")]
#[case("{ |a, b $a + $b }")]
#[case("do { |a $a } 1")]
#[case("do { |a $a } 1 2")]
fn closure_param_list_not_terminated(#[case] input: &str) -> TestResult {
    fail_test(input, "unclosed |")
}
