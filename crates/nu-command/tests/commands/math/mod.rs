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
            1
        "#
    ));

    assert_eq!(actual.out, "1");
}

#[test]
fn add() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            1 + 1
        "#
    ));

    assert_eq!(actual.out, "2");
}

#[test]
fn add_compound() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            1 + 2 + 2
        "#
    ));

    assert_eq!(actual.out, "5");
}

#[test]
fn precedence_of_operators() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            1 + 2 * 2
        "#
    ));

    assert_eq!(actual.out, "5");
}

#[test]
fn precedence_of_operators2() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            1 + 2 * 2 + 1
        "#
    ));

    assert_eq!(actual.out, "6");
}

#[test]
fn precedence_of_operators3() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            5 - 5 * 10 + 5
        "#
    ));

    assert_eq!(actual.out, "-40");
}

#[test]
fn precedence_of_operators4() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            5 - (5 * 10) + 5
        "#
    ));

    assert_eq!(actual.out, "-40");
}

#[test]
fn division_of_ints() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            4 / 2
        "#
    ));

    assert_eq!(actual.out, "2");
}

#[test]
fn division_of_ints2() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            1 / 4
        "#
    ));

    assert_eq!(actual.out, "0.25");
}

#[test]
fn error_zero_division_int_int() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            1 / 0
        "#
    ));

    assert!(actual.err.contains("division by zero"));
}

#[test]
fn error_zero_division_decimal_int() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            1.0 / 0
        "#
    ));

    assert!(actual.err.contains("division by zero"));
}

#[test]
fn error_zero_division_int_decimal() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            1 / 0.0
        "#
    ));

    assert!(actual.err.contains("division by zero"));
}

#[test]
fn error_zero_division_decimal_decimal() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            1.0 / 0.0
        "#
    ));

    assert!(actual.err.contains("division by zero"));
}

#[test]
fn floor_division_of_ints() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            5 // 2
        "#
    ));

    assert_eq!(actual.out, "2");
}

#[test]
fn floor_division_of_ints2() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            -3 // 2
        "#
    ));

    assert_eq!(actual.out, "-2");
}

#[test]
fn floor_division_of_floats() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            -3.0 // 2.0
        "#
    ));

    assert_eq!(actual.out, "-2");
}

#[test]
fn error_zero_floor_division_int_int() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            1 // 0
        "#
    ));

    assert!(actual.err.contains("division by zero"));
}

#[test]
fn error_zero_floor_division_decimal_int() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            1.0 // 0
        "#
    ));

    assert!(actual.err.contains("division by zero"));
}

#[test]
fn error_zero_floor_division_int_decimal() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            1 // 0.0
        "#
    ));

    assert!(actual.err.contains("division by zero"));
}

#[test]
fn error_zero_floor_division_decimal_decimal() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            1.0 // 0.0
        "#
    ));

    assert!(actual.err.contains("division by zero"));
}
#[test]
fn proper_precedence_history() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            2 / 2 / 2 + 1
        "#
    ));

    assert_eq!(actual.out, "1.5");
}

#[test]
fn parens_precedence() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            4 * (6 - 3)
        "#
    ));

    assert_eq!(actual.out, "12");
}

#[test]
fn modulo() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            9 mod 2
        "#
    ));

    assert_eq!(actual.out, "1");
}

#[test]
fn unit_multiplication_math() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            1mb * 2
        "#
    ));

    assert_eq!(actual.out, "1.9 MiB");
}

#[test]
fn unit_multiplication_float_math() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            1mb * 1.2
        "#
    ));

    assert_eq!(actual.out, "1.1 MiB");
}

#[test]
fn unit_float_floor_division_math() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            1mb // 3.0
        "#
    ));

    assert_eq!(actual.out, "325.5 KiB");
}

#[test]
fn unit_division_math() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            1mb / 4
        "#
    ));

    assert_eq!(actual.out, "244.1 KiB");
}

#[test]
fn unit_float_division_math() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            1mb / 3.1
        "#
    ));

    assert_eq!(actual.out, "315.0 KiB");
}

#[test]
fn duration_math() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            1wk + 1day
        "#
    ));

    assert_eq!(actual.out, "1wk 1day");
}

#[test]
fn duration_decimal_math() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            5.5day + 0.5day
        "#
    ));

    assert_eq!(actual.out, "6day");
}

#[test]
fn duration_math_with_nanoseconds() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            1wk + 10ns
        "#
    ));

    assert_eq!(actual.out, "1wk 10ns");
}

#[test]
fn duration_decimal_math_with_nanoseconds() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            1.5wk + 10ns
        "#
    ));

    assert_eq!(actual.out, "1wk 3day 10ns");
}

#[test]
fn duration_decimal_math_with_all_units() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            1wk + 3day + 8hr + 10min + 16sec + 121ms + 11us + 12ns
        "#
    ));

    assert_eq!(actual.out, "1wk 3day 8hr 10min 16sec 121ms 11µs 12ns");
}

#[test]
fn duration_decimal_dans_test() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            3.14sec
        "#
    ));

    assert_eq!(actual.out, "3sec 140ms");
}

#[test]
fn duration_math_with_negative() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            1day - 1wk
        "#
    ));

    assert_eq!(actual.out, "-6day");
}

#[test]
fn compound_comparison() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            4 > 3 && 2 > 1
        "#
    ));

    assert_eq!(actual.out, "true");
}

#[test]
fn compound_comparison2() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            4 < 3 || 2 > 1
        "#
    ));

    assert_eq!(actual.out, "true");
}

#[test]
fn compound_where() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            echo '[{"a": 1, "b": 1}, {"a": 2, "b": 1}, {"a": 2, "b": 2}]' | from json | where a == 2 && b == 1 | to json -r
        "#
    ));

    assert_eq!(actual.out, r#"[{"a": 2,"b": 1}]"#);
}

#[test]
fn compound_where_paren() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            echo '[{"a": 1, "b": 1}, {"a": 2, "b": 1}, {"a": 2, "b": 2}]' | from json | where ($it.a == 2 && $it.b == 1) || $it.b == 2 | to json -r
        "#
    ));

    assert_eq!(actual.out, r#"[{"a": 2,"b": 1},{"a": 2,"b": 2}]"#);
}

#[test]
fn adding_lists() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            [1 3] ++ [5 6] | to nuon
        "#
    ));

    assert_eq!(actual.out, "[1, 3, 5, 6]");
}

#[test]
fn adding_list_and_value() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            [1 3] ++ 5 | to nuon
        "#
    ));

    assert_eq!(actual.out, "[1, 3, 5]");
}

#[test]
fn adding_value_and_list() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            1 ++ [3 5] | to nuon
        "#
    ));

    assert_eq!(actual.out, "[1, 3, 5]");
}
