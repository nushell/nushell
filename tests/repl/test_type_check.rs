use std::collections::HashMap;

use miette::Diagnostic;
use nu_experimental::ENFORCE_RUNTIME_ANNOTATIONS;
use nu_test_support::prelude::*;
use pretty_assertions::assert_matches;
use rstest::rstest;

#[rstest]
#[case::chained_operator_typecheck("1 != 2 and 3 != 4 and 5 != 6", true)]
#[case::type_in_list_of_this_type("42 in [41 42 43]", true)]
#[case::number_int("def foo [x:number] { $x }; foo 1", 1)]
#[case::number_float("def foo [x:number] { $x }; foo 1.4", 1.4)]
#[case::date_minus_duration("2023-04-22 - 2day | format date %Y-%m-%d", "2023-04-20")]
#[case::date_plus_duration("2023-04-18 + 2day | format date %Y-%m-%d", "2023-04-20")]
#[case::duration_plus_date(
    "2024-11-10T00:00:00-00:00 + 4hr | format date",
    "Sun, 10 Nov 2024 04:00:00 +0000"
)]
#[case::record_subtyping(
    "def test [rec: record<name: string, age: int>] { $rec | describe };
    test { age: 4, name: 'John' }",
    "record<age: int, name: string>"
)]
#[case::record_subtyping_2(
    "def test [rec: record<name: string, age: int>] { $rec | describe };
    test { age: 4, name: 'John', height: '5-9' }",
    "record<age: int, name: string, height: string>"
)]
#[case::record_subtyping_allows_general_record(
    "def test []: record<name: string, age: int> -> string { $in; 'success' };
    def underspecified []: nothing -> record {{name:'Douglas', age:42}};
    underspecified | test",
    "success"
)]
#[case::record_subtyping_allows_record_after_general_command(
    "def test []: record<name: string, age: int> -> string { $in; 'success' };
    {name:'Douglas', surname:'Adams', age:42} | select name age | test",
    "success"
)]
fn successful_typecheck_cases(#[case] input: &str, #[case] expected: impl IntoValue) -> Result {
    test().run(input).expect_value_eq(expected)
}

#[rstest]
#[case::record_subtyping_allows_general_inner(
    "def merge_records [other: record<bar: int>]: record<foo: string> -> record<foo: string, bar: int> { merge $other }"
)]
#[case::record_subtyping_works(
    r#"def merge_records [other: record<bar: int>] { "" }; merge_records {"bar": 3, "foo": 4}"#
)]
#[case::in_variable_expression_correct_output_type(
    r#"def foo []: nothing -> string { 'foo' | $"($in)" }"#
)]
fn successful_typecheck_without_output(#[case] input: &str) -> Result {
    let _: Value = test().run(input)?;
    Ok(())
}

#[rstest]
#[case::type_in_list_of_non_this_type(
    "'hello' in [41 42 43]",
    "nu::parser::operator_incompatible_types"
)]
#[case::duration_minus_date_not_supported(
    "2day - 2023-04-22",
    "nu::parser::operator_incompatible_types"
)]
#[case::pipeline_input_on_rhs_is_type_checked(
    r#"def test []: int -> any { "x" + $in }; 3 | test"#,
    "nu::parser::operator_incompatible_types"
)]
#[case::array_of_wrong_types(
    "0..128 | each {} | into string | bytes collect",
    "nu::shell::only_supports_this_input_type"
)]
fn failing_typecheck_error_code_cases(#[case] input: &str, #[case] expected: &str) -> Result {
    test().run(input).expect_error_code_eq(expected)
}

#[rstest]
#[case::int_record_mismatch("def foo [x:int] { $x }; foo {}", "int", "record")]
#[case::record_subtyping_3(
    "def test [rec: record<name: string, age: int>] { $rec | describe };
    test { name: 'Nu' }",
    "record<name: string, age: int>",
    "record<name: string>"
)]
fn failing_typecheck_type_mismatch_cases(
    #[case] input: &str,
    #[case] expected_type: &str,
    #[case] found_type: &str,
) -> Result {
    let err = test().run(input).expect_parse_error()?;

    assert_matches!(
        err,
        ParseError::TypeMismatch(expected, found, _)
            if expected.to_string() == expected_type && found.to_string() == found_type
    );
    Ok(())
}

#[rstest]
#[case::block_not_first_class_def(
    "def foo [x: block] { do $x }",
    "Blocks are not support as first-class values"
)]
#[case::block_not_first_class_let(
    "let x: block = { 3 }",
    "Blocks are not support as first-class values"
)]
fn block_types_are_not_first_class(#[case] input: &str, #[case] expected_error: &str) -> Result {
    let err = test().run(input).expect_parse_error()?;

    assert_matches!(
        err,
        ParseError::LabeledErrorWithHelp { error, .. } if error == expected_error
    );
    Ok(())
}

#[test]
fn in_variable_expression_wrong_output_type() -> Result {
    let err = test()
        .run(r#"def foo []: nothing -> int { 'foo' | $"($in)" }"#)
        .expect_parse_error()?;

    assert_matches!(
        err,
        ParseError::OutputMismatch(expected, actual, _)
            if expected.to_string() == "int" && actual == "string"
    );
    Ok(())
}

#[test]
fn in_oneof_block_expected_block() -> Result {
    let err = test()
        .run("match 1 { 0 => { try 3 } }")
        .expect_parse_error()?;

    assert_matches!(err, ParseError::Expected("block, closure or record", _));
    Ok(())
}

#[rstest]
// [ int, number ] is widened to list<number>
#[case("let n: number = 1; let foo = [ 1, $n ];", "list<number>")]
// supertype of list elements (records)
#[case("let foo = [ { a: 1 }, { a: 1, b: 2 } ];", "list<record<a: int>>")]
// [ list supertype, table ]
#[case(
    "let foo = [ [ { a: 1 } ], [ [a, b]; [1, 2] ] ];",
    "list<oneof<list<record<a: int>>, table<a: int, b: int>>>"
)]
// [ list, table supertype ]
#[case(
    "let foo = [ [{ a: 1, b: 2 }], [ [a]; [1] ] ];",
    "list<oneof<list<record<a: int, b: int>>, table<a: int>>>"
)]
// disjoint element types: empty element supertype
#[case(
    "let foo = [[ [bar]; [1] ], [ { baz: 1 } ] ];",
    "list<oneof<table<bar: int>, list<record<baz: int>>>>"
)]
// `bar: int` and `bar: number` are widened to table<bar: number>
#[case(
    "let n: number = 1; let foo = [ [bar]; [1], [$n] ];",
    "table<bar: number>"
)]
// supertype of table values (records)
#[case(
    "let foo = [ [item]; [ {a: 1} ], [ {a: 1, b: 1 } ] ];",
    "table<item: record<a: int>>"
)]
// disjoint table values: oneof
#[case("let foo = [ [bar]; [1], [true] ];", "table<bar: oneof<int, bool>>")]
#[case(
    "let a: any = 1; let b: int = 2; let foo = [ [bar]; [$a], [$b] ];",
    "table<bar: any>"
)]
fn collection_supertype_inference(#[case] assignment: &str, #[case] expected_type: &str) -> Result {
    test()
        .run(format!(
            r#"{assignment} scope variables | where name == "$foo" | first | get type"#
        ))
        .expect_value_eq(expected_type)
}

#[rstest]
#[case::empty(
    "def f []: [oneof<int, nothing> -> nothing] { describe }; f",
    "nothing"
)]
#[case::byte_stream(
    "def f []: [oneof<int, binary> -> nothing] { describe }; [0x[01]] | bytes collect | f",
    "binary (stream)"
)]
#[case::list_stream(
    "def f []: [oneof<string, list<int>> -> nothing] { describe }; [1] | each {} | f",
    "list<int> (stream)"
)]
fn pipeline_oneof(#[case] input: &str, #[case] expected: &str) -> Result {
    test().run(input).expect_value_eq(expected)
}

#[rstest]
#[case::filter_output_union(
    "
        let pending = ([a] | each {} | collect | skip 0)
        for item in $pending {}
    "
)]
#[case::union_of_iterables(
    "
        def choose []: nothing -> oneof<list<list<string>>, list<int>> {
            [[a]]
        }
        for item in (choose) {}
    "
)]
#[case::static_list_source(
    "
        let pending: oneof<table, binary, list<int>> = [1 2 3]
        for item in $pending {}
    "
)]
#[case::static_table_source(
    "
        let pending: oneof<table, binary, list<int>> = [[a b]; [1 2], [3 4]]
        for item in $pending {}
    "
)]
#[case::static_binary_source(
    "
        let pending: oneof<table, binary, list<int>> = 0x[deadbeef]
        for item in $pending {}
    "
)]
#[test]
#[exp(ENFORCE_RUNTIME_ANNOTATIONS)]
fn for_loop_item_type_from_iterable_union(#[case] input: &str) -> Result {
    // should return nothing
    let () = test().run(input)?;
    Ok(())
}

#[test]
#[exp(ENFORCE_RUNTIME_ANNOTATIONS)]
fn for_loop_incorrect_type_raises_error() -> Result {
    let code = "
        def incorrectly_typed_stream []: nothing -> list<int> {
            # using `each`:
            # - erases the type: bypassing parse time type checking
            # - returns a stream rather than a value: bypassing runtime type checking
            [a b c] | each {}
        }

        for item in (incorrectly_typed_stream) {}
    ";
    let err = test().run(code).expect_shell_error()?;

    assert_eq!(err.code().unwrap().to_string(), "nu::shell::type_mismatch");

    let labels = err
        .labels()
        .into_iter()
        .flatten()
        .filter_map(|label| label.label().map(String::from))
        .collect::<Vec<_>>();

    assert_contains("the value is a string".to_string(), &labels);
    assert_contains("expected int, got string".to_string(), &labels);

    Ok(())
}

#[test]
fn transpose_into_load_env() -> Result {
    test()
        .run(
            "[[col1, col2]; [a, 10], [b, 20]] | transpose --ignore-titles -r -d | load-env; $env.a",
        )
        .expect_value_eq(10)
}

#[rstest]
#[case("if true {} else { foo 1 }")]
#[case("if true {} else if (foo 1) == null { }")]
#[case("match 1 { 0 => { foo 1 } }")]
#[case("try { } catch { foo 1 }")]
/// type errors should propagate from `OneOf(Block | Closure | Expression, ..)`
fn in_oneof_block_expected_type(#[case] input: &str) -> Result {
    let def = "def foo [bar: bool] {};";
    let err = test().run(format!("{def} {input}")).expect_parse_error()?;

    assert_matches!(err, ParseError::ExpectedWithStringMsg(expected, _) if expected == "bool");
    Ok(())
}

#[test]
fn pipeline_multiple_types() -> Result {
    // https://github.com/nushell/nushell/issues/15485
    let actual: String = test().run("{year: 2019} | into datetime | date humanize")?;
    assert_contains("years ago", actual);
    Ok(())
}

const MULTIPLE_TYPES_DEFS: &str = "
    def foo []: [int -> int, int -> string] {
        if $in > 2 { 'hi' } else 4
    }

    def bar []: [int -> filesize, string -> string] {
        if $in == 'hi' { 'meow' } else { into filesize }
    }
";

#[rstest]
#[case::custom("5 | foo | str trim", "hi")]
#[case::propagate_string("5 | foo | bar | str trim", "meow")]
#[case::propagate_int("2 | foo | bar | format filesize B", "4 B")]
fn pipeline_multiple_types_propagates(#[case] pipeline: &str, #[case] expected: &str) -> Result {
    test()
        .run(format!("{MULTIPLE_TYPES_DEFS}{pipeline}"))
        .expect_value_eq(expected)
}

#[test]
fn pipeline_multiple_types_propagate_error() -> Result {
    test()
        .run(format!(
            "{MULTIPLE_TYPES_DEFS}
            2 | foo | bar | values"
        ))
        .expect_error_code_eq("nu::parser::input_type_mismatch")
}

#[test]
#[exp(ENFORCE_RUNTIME_ANNOTATIONS)]
fn optional_parameters_and_flags_are_nullable() -> Result {
    let mut tester = test();

    let code = "
        def foo [opt_param?: int] {
            let var = $opt_param
        }
        foo
    ";
    let () = tester.run(code)?;

    let code = "
        def foo [--flag: int] {
            let var = $flag
        }
        foo
    ";
    let () = tester.run(code)?;

    Ok(())
}

#[test]
fn pipeline_let_type() -> Result {
    let mut tester = test();

    let code = "
        [1, 2, 3]
        | let list_int
        | into string
        | let list_str
        | str join
        | let str
    ";
    let _: Value = tester.run(code)?;

    let code = "scope variables | select name type | transpose -dr";
    let out: HashMap<String, String> = tester.run(code)?;

    assert_eq!(out["$list_int"], "list<int>");
    assert_eq!(out["$list_str"], "list<string>");
    assert_eq!(out["$str"], "string");

    Ok(())
}

#[test]
fn block_let_rhs_pipeline_input() -> Result {
    let code = r#"
        def only-nothing []: nothing -> string { "hello" }
        def foo []: int -> any { let x = only-nothing; $x + 1 }
        5 | foo
    "#;

    let err = test().run(code).expect_parse_error()?;
    assert!(matches!(err, ParseError::InputMismatch(ty, _) if ty == "int"));

    Ok(())
}

#[test]
fn closure_body_input_type_not_inherited_from_pipeline_input() -> Result {
    let mut tester = test();

    let () = tester.run("let fn = 42 | {|| $in ++ 'kB'}")?;
    tester.run("'10' | do $fn").expect_value_eq("10kB")
}

#[test]
fn closure_body_input_type_not_inherited_from_surrounding_command() -> Result {
    let mut tester = test();

    let code = r#"
        def cmd [p: string]: nothing -> string {
            let fn = {|| $in ++ "bar"}
            $p | do $fn
        }
    "#;

    let () = tester.run(code)?;
    tester.run("cmd foo").expect_value_eq("foobar")
}
