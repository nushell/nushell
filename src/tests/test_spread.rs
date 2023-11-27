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
    run_test(r#"[1 ...[]] | describe"#, "list<int>").unwrap();
    run_test(r#"[1 ...[2]] | describe"#, "list<int>").unwrap();
    run_test(r#"["foo" ...[4 5 6]] | describe"#, "list<any>").unwrap();
    run_test(r#"[1 2 ...["misfit"] 4] | describe"#, "list<any>")
}

#[test]
fn spread_in_record() -> TestResult {
    run_test(r#"{...{}} | to nuon"#, "{}")
}

#[test]
fn duplicate_cols() -> TestResult {
    fail_test(r#"{a: 1, ...{a: 3}} | to nuon"#, "column_defined_twice").unwrap();
    fail_test(r#"{...{a: 3}, a: 1} | to nuon"#, "column_defined_twice").unwrap();
    fail_test(r#"{...{a: 3}, ...{a: 5}}"#, "column_defined_twice")
}

#[test]
fn bad_spread_on_non_record() -> TestResult {
    fail_test(r#"let x = 5; { ...$x }"#, "cannot spread").unwrap();
    fail_test(r#"{...([1, 2])}"#, "cannot spread")
}
