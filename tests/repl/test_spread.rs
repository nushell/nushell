use crate::repl::tests::{TestResult, fail_test, run_test};
use nu_test_support::nu;

#[test]
fn spread_in_list() -> TestResult {
    run_test(r#"[...[]] | to nuon"#, "[]").unwrap();
    run_test(
        r#"[1 2 ...[[3] {x: 1}] 5] | to nuon"#,
        "[1, 2, [3], {x: 1}, 5]",
    )
    .unwrap();
    run_test(
        r#"[...("foo" | split chars) 10] | to nuon"#,
        "[f, o, o, 10]",
    )
    .unwrap();
    run_test(
        r#"let l = [1, 2, [3]]; [...$l $l] | to nuon"#,
        "[1, 2, [3], [1, 2, [3]]]",
    )
    .unwrap();
    run_test(
        r#"[ ...[ ...[ ...[ a ] b ] c ] d ] | to nuon"#,
        "[a, b, c, d]",
    )
}

#[test]
fn not_spread() -> TestResult {
    run_test(r#"def ... [x] { $x }; ... ..."#, "...").unwrap();
    run_test(
        r#"let a = 4; [... $a ... [1] ... (5) ...bare ...] | to nuon"#,
        r#"["...", 4, "...", [1], "...", 5, "...bare", "..."]"#,
    )
}

#[test]
fn bad_spread_on_non_list() -> TestResult {
    fail_test(r#"let x = 5; [...$x]"#, "cannot spread").unwrap();
    fail_test(r#"[...({ x: 1 })]"#, "cannot spread")
}

#[test]
fn spread_type_list() -> TestResult {
    run_test(
        r#"def f [a: list<int>] { $a | describe }; f [1 ...[]]"#,
        "list<int>",
    )
    .unwrap();
    run_test(
        r#"def f [a: list<int>] { $a | describe }; f [1 ...[2]]"#,
        "list<int>",
    )
    .unwrap();
    fail_test(
        r#"def f [a: list<int>] { }; f ["foo" ...[4 5 6]]"#,
        "expected int",
    )
    .unwrap();
    fail_test(
        r#"def f [a: list<int>] { }; f [1 2 ...["misfit"] 4]"#,
        "expected int",
    )
}

#[test]
fn spread_in_record() -> TestResult {
    run_test(r#"{...{} ...{}, a: 1} | to nuon"#, "{a: 1}").unwrap();
    run_test(r#"{...{...{...{}}}} | to nuon"#, "{}").unwrap();
    run_test(
        r#"{foo: bar ...{a: {x: 1}} b: 3} | to nuon"#,
        "{foo: bar, a: {x: 1}, b: 3}",
    )
}

#[test]
fn duplicate_cols() -> TestResult {
    fail_test(r#"{a: 1, ...{a: 3}}"#, "column used twice").unwrap();
    fail_test(r#"{...{a: 4, x: 3}, x: 1}"#, "column used twice").unwrap();
    fail_test(r#"{...{a: 0, x: 2}, ...{x: 5}}"#, "column used twice")
}

#[test]
fn bad_spread_on_non_record() -> TestResult {
    fail_test(r#"let x = 5; { ...$x }"#, "cannot spread").unwrap();
    fail_test(r#"{...([1, 2])}"#, "cannot spread")
}

#[test]
fn spread_type_record() -> TestResult {
    run_test(
        r#"def f [a: record<x: int>] { $a.x }; f { ...{x: 0} }"#,
        "0",
    )
    .unwrap();
    fail_test(
        r#"def f [a: record<x: int>] {}; f { ...{x: "not an int"} }"#,
        "type_mismatch",
    )
}

#[test]
fn spread_external_args() {
    assert_eq!(
        nu!(r#"nu --testbin cococo ...[1 "foo"] 2 ...[3 "bar"]"#).out,
        "1 foo 2 3 bar",
    );
    // exec doesn't have rest parameters but allows unknown arguments
    assert_eq!(
        nu!(r#"exec nu --testbin cococo "foo" ...[5 6]"#).out,
        "foo 5 6"
    );
}

#[test]
fn spread_internal_args() -> TestResult {
    run_test(
        r#"
        let list = ["foo" 4]
        def f [a b c? d? ...x] { [$a $b $c $d $x] | to nuon }
        f 1 2 ...[5 6] 7 ...$list"#,
        "[1, 2, null, null, [5, 6, 7, foo, 4]]",
    )
    .unwrap();
    run_test(
        r#"
        def f [a b c? d? ...x] { [$a $b $c $d $x] | to nuon }
        f 1 2 3 ...[5 6]"#,
        "[1, 2, 3, null, [5, 6]]",
    )
    .unwrap();
    run_test(
        r#"
        def f [--flag: int ...x] { [$flag $x] | to nuon }
        f 2 ...[foo] 4 --flag 5 6 ...[7 8]"#,
        "[5, [2, foo, 4, 6, 7, 8]]",
    )
    .unwrap();
    run_test(
        r#"
        def f [a b? --flag: int ...x] { [$a $b $flag $x] | to nuon }
        f 1 ...[foo] 4 --flag 5 6 ...[7 8]"#,
        "[1, null, 5, [foo, 4, 6, 7, 8]]",
    )
}

#[test]
fn bad_spread_internal_args() -> TestResult {
    fail_test(
        r#"
        def f [a b c? d? ...x] { echo $a $b $c $d $x }
        f 1 ...[5 6]"#,
        "Missing required positional argument",
    )
    .unwrap();
    fail_test(
        r#"
        def f [a b?] { echo a b c d }
        f ...[5 6]"#,
        "unexpected spread argument",
    )
}

#[test]
fn spread_non_list_args() {
    fail_test(r#"echo ...(1)"#, "cannot spread value").unwrap();
    assert!(
        nu!(r#"nu --testbin cococo ...(1)"#)
            .err
            .contains("cannot spread value")
    );
}

#[test]
fn spread_args_type() -> TestResult {
    fail_test(r#"def f [...x: int] {}; f ...["abc"]"#, "expected int")
}

#[test]
fn explain_spread_args() -> TestResult {
    run_test(
        r#"(explain { || echo ...[1 2] }).cmd_args.0 | select arg_type name type | to nuon"#,
        r#"[[arg_type, name, type]; [spread, "[1 2]", list<int>]]"#,
    )
}

#[test]
fn disallow_implicit_spread_for_externals() -> TestResult {
    fail_test(r#"^echo [1 2]"#, "Lists are not automatically spread")
}

#[test]
fn respect_shape() -> TestResult {
    fail_test(
        "def foo [...rest] { ...$rest }; foo bar baz",
        "Command `...$rest` not found",
    )
    .unwrap();
    fail_test("module foo { ...$bar }", "expected_keyword").unwrap();
    run_test(r#"def "...$foo" [] {2}; do { ...$foo }"#, "2").unwrap();
    run_test(r#"match "...$foo" { ...$foo => 5 }"#, "5")
}

#[test]
fn spread_null() -> TestResult {
    // Spread in list
    run_test(r#"[1, 2, ...(null)] | to nuon --raw"#, r#"[1,2]"#)?;

    // Spread in record
    run_test(r#"{a: 1, b: 2, ...(null)} | to nuon --raw"#, r#"{a:1,b:2}"#)?;

    // Spread to built-in command's ...rest
    run_test(r#"echo 1 2 ...(null) | to nuon --raw"#, r#"[1,2]"#)?;

    // Spread to custom command's ...rest
    run_test(
        r#"
            def foo [...rest] { $rest }
            foo ...(null) 1 2 ...(null) 3 | to nuon --raw
        "#,
        r#"[1,2,3]"#,
    )?;

    // Spread to external command's arguments
    assert_eq!(nu!(r#"nu --testbin cococo 1 ...(null) 2"#).out, "1 2");

    Ok(())
}
