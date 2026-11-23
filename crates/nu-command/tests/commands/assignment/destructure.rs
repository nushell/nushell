use nu_test_support::prelude::*;
use pretty_assertions::assert_matches;
use rstest::rstest;

#[rstest]
#[case::swap("mut a = 1; mut b = 2; [$a, $b] = [$b, $a]; [$a $b]", test_value!([2, 1]))]
#[case::record("mut x = 0; mut y = 0; {$x, $y} = {x: 3, y: 4}; [$x $y]", test_value!([3, 4]))]
#[case::renamed_field("mut n = 0; {count: $n} = {count: 5}; $n", test_value!(5))]
#[case::list_rest("mut h = 0; mut t = []; [$h, ..$t] = [1 2 3]; [$h $t]", test_value!([1, [2, 3]]))]
#[case::record_rest("mut x = 0; mut r = {}; {$x, ..$r} = {x: 3, z: 9}; [$x $r]", test_value!([3, {z: 9}]))]
#[case::default_value("mut a = 0; mut b = 0; [$a, $b = 7] = [5]; [$a $b]", test_value!([5, 7]))]
#[case::range("mut a = 0; [$a, ..] = 1..5; $a", test_value!(1))]
#[case::in_blocks(
    "def f [] {
        mut a = 0
        mut b = 0
        for pair in [[1 2] [3 4]] { [$a, $b] = $pair }
        if true { [$a, $b] = [$b, $a] }
        mut i = 0
        while $i < 1 { [$a] = [9]; $i += 1 }
        [$a $b]
    }
    f",
    test_value!([9, 3])
)]
fn destructuring_assignment(#[case] code: &str, #[case] expected: Value) -> Result {
    test().run(code).expect_value_eq(expected)
}

#[test]
fn failed_destructuring_assignment_changes_nothing() -> Result {
    test()
        .run("mut a = 1; mut b = 2; try { [$a, $b] = [5] }; [$a $b]")
        .expect_value_eq(test_value!([1, 2]))
}

#[test]
fn destructuring_assignment_mismatch() -> Result {
    let err = test()
        .run("mut a = 1; mut b = 2; [$a, $b] = [5]")
        .expect_shell_error()?;
    assert_matches!(
        err,
        ShellError::PatternMismatch { msg, .. } if msg == "list has 1 item, the pattern takes 2"
    );
    Ok(())
}

#[rstest]
#[case::immutable(
    "let a = 1; [$a] = [2]",
    "nu::parser::assignment_requires_mutable_variable"
)]
#[case::not_found("mut a = 1; [$a, $nope] = [1 2]", "nu::parser::variable_not_found")]
#[case::variable_type("mut x: int = 1; [$x] = ['a']", "nu::parser::type_mismatch")]
#[case::shape("mut x = 1; {$x} = 5", "nu::parser::type_mismatch")]
#[case::captured_in_closure("mut a = 1; do { [$a] = [2] }", "nu::parser::expected_keyword")]
fn destructuring_assignment_errors(#[case] code: &str, #[case] expected: &str) -> Result {
    test().run(code).expect_error_code_eq(expected)
}

#[rstest]
#[case::compound_operator("mut a = 1; [$a] += [2]", "Only `=` can assign to a pattern")]
#[case::bound_twice("mut a = 1; [$a, $a] = [1 2]", "`$a` is bound twice in this pattern")]
#[case::literal(
    "mut a = 1; [$a, 1] = [1 1]",
    "A destructuring pattern can't hold a literal"
)]
fn destructuring_assignment_invalid(#[case] code: &str, #[case] expected: &str) -> Result {
    let err = test().run(code).expect_parse_error()?;
    assert_matches!(err, ParseError::LabeledErrorWithHelp { error, .. } if error == expected);
    Ok(())
}
