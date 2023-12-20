use crate::tests::{fail_test, run_test, TestResult};

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
fn const_spread_in_list() -> TestResult {
    run_test(r#"const x = [...[]]; $x | to nuon"#, "[]").unwrap();
    run_test(
        r#"const x = [1 2 ...[[3] {x: 1}] 5]; $x | to nuon"#,
        "[1, 2, [3], {x: 1}, 5]",
    )
    .unwrap();
    run_test(
        r#"const x = [...([f o o]) 10]; $x | to nuon"#,
        "[f, o, o, 10]",
    )
    .unwrap();
    run_test(
        r#"const l = [1, 2, [3]]; const x = [...$l $l]; $x | to nuon"#,
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
        "[..., 4, ..., [1], ..., 5, ...bare, ...]",
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
    run_test(r#"{...{...{...{}}}} | to nuon"#, "{}").unwrap();
    run_test(
        r#"{foo: bar ...{a: {x: 1}} b: 3} | to nuon"#,
        "{foo: bar, a: {x: 1}, b: 3}",
    )
}

#[test]
fn const_spread_in_record() -> TestResult {
    run_test(r#"const x = {...{...{...{}}}}; $x | to nuon"#, "{}").unwrap();
    run_test(
        r#"const x = {foo: bar ...{a: {x: 1}} b: 3}; $x | to nuon"#,
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
fn const_duplicate_cols() -> TestResult {
    fail_test(r#"const _ = {a: 1, ...{a: 3}}"#, "column used twice").unwrap();
    fail_test(r#"const _ = {...{a: 4, x: 3}, x: 1}"#, "column used twice").unwrap();
    fail_test(
        r#"const _ = {...{a: 0, x: 2}, ...{x: 5}}"#,
        "column used twice",
    )
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
