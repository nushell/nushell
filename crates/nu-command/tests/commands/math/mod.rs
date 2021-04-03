mod avg;
mod eval;
mod median;
mod round;
mod sqrt;
mod sum;

use nu_test_support::{nu, pipeline};

#[test]
fn one_arg() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            = 1
        "#
    ));

    assert_eq!(actual.out, "1");
}

#[test]
fn add() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            = 1 + 1
        "#
    ));

    assert_eq!(actual.out, "2");
}

#[test]
fn add_compound() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            = 1 + 2 + 2
        "#
    ));

    assert_eq!(actual.out, "5");
}

#[test]
fn precedence_of_operators() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            = 1 + 2 * 2
        "#
    ));

    assert_eq!(actual.out, "5");
}

#[test]
fn precedence_of_operators2() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            = 1 + 2 * 2 + 1
        "#
    ));

    assert_eq!(actual.out, "6");
}

#[test]
fn division_of_ints() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            = 4 / 2
        "#
    ));

    assert_eq!(actual.out, "2");
}

#[test]
fn division_of_ints2() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            = 1 / 4
        "#
    ));

    assert_eq!(actual.out, "0.25");
}

#[test]
fn error_zero_division_int_int() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            = 1 / 0
        "#
    ));

    assert!(actual.err.contains("division by zero"));
}

#[test]
fn error_zero_division_decimal_int() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            = 1.0 / 0
        "#
    ));

    assert!(actual.err.contains("division by zero"));
}

#[test]
fn error_zero_division_int_decimal() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            = 1 / 0.0
        "#
    ));

    assert!(actual.err.contains("division by zero"));
}

#[test]
fn error_zero_division_decimal_decimal() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            = 1.0 / 0.0
        "#
    ));

    assert!(actual.err.contains("division by zero"));
}

#[test]
fn proper_precedence_history() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            = 2 / 2 / 2 + 1
        "#
    ));

    assert_eq!(actual.out, "1.5");
}

#[test]
fn parens_precedence() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            = 4 * (6 - 3)
        "#
    ));

    assert_eq!(actual.out, "12");
}

#[test]
fn modulo() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            = 9 mod 2
        "#
    ));

    assert_eq!(actual.out, "1");
}

#[test]
fn duration_math() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            = 1wk + 1day
        "#
    ));

    assert_eq!(actual.out, "8day");
}

#[test]
fn duration_decimal_math() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            = 0.5mon + 1day
        "#
    ));

    assert_eq!(actual.out, "16day");
}

#[test]
fn duration_math_with_nanoseconds() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            = 1wk + 10ns
        "#
    ));

    assert_eq!(actual.out, "7day 10ns");
}

#[test]
fn duration_decimal_math_with_nanoseconds() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            = 1.5wk + 10ns
        "#
    ));

    assert_eq!(actual.out, "10day 10ns");
}

#[test]
fn duration_math_with_negative() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            = 1day - 1wk
        "#
    ));

    assert_eq!(actual.out, "-6day");
}

#[test]
fn compound_comparison() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            = 4 > 3 && 2 > 1
        "#
    ));

    assert_eq!(actual.out, "true");
}

#[test]
fn compound_comparison2() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            = 4 < 3 || 2 > 1
        "#
    ));

    assert_eq!(actual.out, "true");
}

#[test]
fn compound_where() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            echo '[{"a": 1, "b": 1}, {"a": 2, "b": 1}, {"a": 2, "b": 2}]' | from json | where a == 2 && b == 1 | to json
        "#
    ));

    assert_eq!(actual.out, r#"{"a":2,"b":1}"#);
}

#[test]
fn compound_where_paren() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            echo '[{"a": 1, "b": 1}, {"a": 2, "b": 1}, {"a": 2, "b": 2}]' | from json | where (a == 2 && b == 1) || b == 2 | to json
        "#
    ));

    assert_eq!(actual.out, r#"[{"a":2,"b":1},{"a":2,"b":2}]"#);
}
