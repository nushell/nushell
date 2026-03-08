mod abs;
mod avg;
mod ceil;
mod floor;
mod log;
mod max;
mod median;
mod min;
mod mode;
mod product;
mod round;
mod sqrt;
mod stddev;
mod sum;
mod variance;

use nu_test_support::prelude::*;

#[test]
fn one_arg() -> Result {
    let code = r#"
        1
    "#;
    let outcome: i64 = test().run(code)?;

    assert_eq!(outcome, 1);
    Ok(())
}

#[test]
fn add() -> Result {
    let code = r#"
        1 + 1
    "#;
    let outcome: i64 = test().run(code)?;

    assert_eq!(outcome, 2);
    Ok(())
}

#[test]
fn add_compound() -> Result {
    let code = r#"
        1 + 2 + 2
    "#;
    let outcome: i64 = test().run(code)?;

    assert_eq!(outcome, 5);
    Ok(())
}

#[test]
fn precedence_of_operators() -> Result {
    let code = r#"
        1 + 2 * 2
    "#;
    let outcome: i64 = test().run(code)?;

    assert_eq!(outcome, 5);
    Ok(())
}

#[test]
fn precedence_of_operators2() -> Result {
    let code = r#"
        1 + 2 * 2 + 1
    "#;
    let outcome: i64 = test().run(code)?;

    assert_eq!(outcome, 6);
    Ok(())
}

#[test]
fn precedence_of_operators3() -> Result {
    let code = r#"
        5 - 5 * 10 + 5
    "#;
    let outcome: i64 = test().run(code)?;

    assert_eq!(outcome, -40);
    Ok(())
}

#[test]
fn precedence_of_operators4() -> Result {
    let code = r#"
        5 - (5 * 10) + 5
    "#;
    let outcome: i64 = test().run(code)?;

    assert_eq!(outcome, -40);
    Ok(())
}

#[test]
fn division_of_ints() -> Result {
    let code = r#"
        4 / 2
    "#;
    let outcome: f64 = test().run(code)?;

    assert_eq!(outcome, 2.0);
    Ok(())
}

#[test]
fn division_of_ints2() -> Result {
    let code = r#"
        1 / 4
    "#;
    let outcome: f64 = test().run(code)?;

    assert_eq!(outcome, 0.25);
    Ok(())
}

#[test]
fn error_zero_division_int_int() -> Result {
    let code = r#"
        1 / 0
    "#;
    let outcome = test().run(code).expect_shell_error()?;

    assert!(matches!(outcome, ShellError::DivisionByZero { .. }));
    Ok(())
}

#[test]
fn error_zero_division_float_int() -> Result {
    let code = r#"
        1.0 / 0
    "#;
    let outcome = test().run(code).expect_shell_error()?;

    assert!(matches!(outcome, ShellError::DivisionByZero { .. }));
    Ok(())
}

#[test]
fn error_zero_division_int_float() -> Result {
    let code = r#"
        1 / 0.0
    "#;
    let outcome = test().run(code).expect_shell_error()?;

    assert!(matches!(outcome, ShellError::DivisionByZero { .. }));
    Ok(())
}

#[test]
fn error_zero_division_float_float() -> Result {
    let code = r#"
        1.0 / 0.0
    "#;
    let outcome = test().run(code).expect_shell_error()?;

    assert!(matches!(outcome, ShellError::DivisionByZero { .. }));
    Ok(())
}

#[test]
fn floor_division_of_ints() -> Result {
    let code = r#"
        5 // 2
    "#;
    let outcome: i64 = test().run(code)?;

    assert_eq!(outcome, 2);
    Ok(())
}

#[test]
fn floor_division_of_ints2() -> Result {
    let code = r#"
        -3 // 2
    "#;
    let outcome: i64 = test().run(code)?;

    assert_eq!(outcome, -2);
    Ok(())
}

#[test]
fn floor_division_of_floats() -> Result {
    let code = r#"
        -3.0 // 2.0
    "#;
    let outcome: f64 = test().run(code)?;

    assert_eq!(outcome, -2.0);
    Ok(())
}

#[test]
fn error_zero_floor_division_int_int() -> Result {
    let code = r#"
        1 // 0
    "#;
    let outcome = test().run(code).expect_shell_error()?;

    assert!(matches!(outcome, ShellError::DivisionByZero { .. }));
    Ok(())
}

#[test]
fn error_zero_floor_division_float_int() -> Result {
    let code = r#"
        1.0 // 0
    "#;
    let outcome = test().run(code).expect_shell_error()?;

    assert!(matches!(outcome, ShellError::DivisionByZero { .. }));
    Ok(())
}

#[test]
fn error_zero_floor_division_int_float() -> Result {
    let code = r#"
        1 // 0.0
    "#;
    let outcome = test().run(code).expect_shell_error()?;

    assert!(matches!(outcome, ShellError::DivisionByZero { .. }));
    Ok(())
}

#[test]
fn error_zero_floor_division_float_float() -> Result {
    let code = r#"
        1.0 // 0.0
    "#;
    let outcome = test().run(code).expect_shell_error()?;

    assert!(matches!(outcome, ShellError::DivisionByZero { .. }));
    Ok(())
}
#[test]
fn proper_precedence_history() -> Result {
    let code = r#"
        2 / 2 / 2 + 1
    "#;
    let outcome: f64 = test().run(code)?;

    assert_eq!(outcome, 1.5);
    Ok(())
}

#[test]
fn parens_precedence() -> Result {
    let code = r#"
        4 * (6 - 3)
    "#;
    let outcome: i64 = test().run(code)?;

    assert_eq!(outcome, 12);
    Ok(())
}

#[test]
fn modulo() -> Result {
    let code = r#"
        9 mod 2
    "#;
    let outcome: i64 = test().run(code)?;

    assert_eq!(outcome, 1);
    Ok(())
}

#[test]
fn floor_div_mod() -> Result {
    let outcome: bool = test().run("let q = 8 // -3; let r = 8 mod -3; 8 == $q * -3 + $r")?;
    assert!(outcome);

    let outcome: bool = test().run("let q = -8 // 3; let r = -8 mod 3; -8 == $q * 3 + $r")?;
    assert!(outcome);
    Ok(())
}

#[test]
fn floor_div_mod_overflow() -> Result {
    let code = format!("{} // -1", i64::MIN);
    let outcome = test().run(&code).expect_shell_error()?;
    assert!(matches!(outcome, ShellError::OperatorOverflow { .. }));

    let code = format!("{} mod -1", i64::MIN);
    let outcome = test().run(&code).expect_shell_error()?;
    assert!(matches!(outcome, ShellError::OperatorOverflow { .. }));
    Ok(())
}

#[test]
fn floor_div_mod_zero() -> Result {
    let outcome = test().run("1 // 0").expect_shell_error()?;
    assert!(matches!(outcome, ShellError::DivisionByZero { .. }));

    let outcome = test().run("1 mod 0").expect_shell_error()?;
    assert!(matches!(outcome, ShellError::DivisionByZero { .. }));
    Ok(())
}

#[test]
fn floor_div_mod_large_num() -> Result {
    let code = format!("{} // {}", i64::MAX, i64::MAX / 2);
    let outcome: i64 = test().run(&code)?;
    assert_eq!(outcome, 2);

    let code = format!("{} mod {}", i64::MAX, i64::MAX / 2);
    let outcome: i64 = test().run(&code)?;
    assert_eq!(outcome, 1);
    Ok(())
}

#[test]
fn unit_multiplication_math() -> Result {
    let outcome: Value = test().run("1MB * 2")?;
    assert_eq!(outcome, Value::test_filesize(2_000_000));
    Ok(())
}

#[test]
fn unit_multiplication_float_math() -> Result {
    let outcome: Value = test().run("1MB * 1.2")?;
    assert_eq!(outcome, Value::test_filesize(1_200_000));
    Ok(())
}

#[test]
fn unit_float_floor_division_math() -> Result {
    let outcome: Value = test().run("1MB // 3.0")?;
    assert_eq!(outcome, Value::test_filesize(333_333));
    Ok(())
}

#[test]
fn unit_division_math() -> Result {
    let outcome: Value = test().run("1MB / 4")?;
    assert_eq!(outcome, Value::test_filesize(250_000));
    Ok(())
}

#[test]
fn unit_float_division_math() -> Result {
    let outcome: Value = test().run("1MB / 3.2")?;
    assert_eq!(outcome, Value::test_filesize(312_500));
    Ok(())
}

#[test]
fn duration_math() -> Result {
    let code = r#"
        1wk + 1day
    "#;
    let outcome: Value = test().run(code)?;

    assert_eq!(outcome, Value::test_duration(691_200_000_000_000));
    Ok(())
}

#[test]
fn duration_decimal_math() -> Result {
    let code = r#"
        5.5day + 0.5day
    "#;
    let outcome: Value = test().run(code)?;

    assert_eq!(outcome, Value::test_duration(518_400_000_000_000));
    Ok(())
}

#[test]
fn duration_math_with_nanoseconds() -> Result {
    let code = r#"
        1wk + 10ns
    "#;
    let outcome: Value = test().run(code)?;

    assert_eq!(outcome, Value::test_duration(604_800_000_000_010));
    Ok(())
}

#[test]
fn duration_decimal_math_with_nanoseconds() -> Result {
    let code = r#"
        1.5wk + 10ns
    "#;
    let outcome: Value = test().run(code)?;

    assert_eq!(outcome, Value::test_duration(907_200_000_000_010));
    Ok(())
}

#[test]
fn duration_decimal_math_with_all_units() -> Result {
    let code = r#"
        1wk + 3day + 8hr + 10min + 16sec + 121ms + 11us + 12ns
    "#;
    let outcome: Value = test().run(code)?;

    assert_eq!(outcome, Value::test_duration(893_416_121_011_012));
    Ok(())
}

#[test]
fn duration_decimal_dans_test() -> Result {
    let code = r#"
        3.14sec
    "#;
    let outcome: Value = test().run(code)?;

    assert_eq!(outcome, Value::test_duration(3_140_000_000));
    Ok(())
}

#[test]
fn duration_math_with_negative() -> Result {
    let code = r#"
        1day - 1wk
    "#;
    let outcome: Value = test().run(code)?;

    assert_eq!(outcome, Value::test_duration(-518_400_000_000_000));
    Ok(())
}

#[test]
fn compound_comparison() -> Result {
    let code = r#"
        4 > 3 and 2 > 1
    "#;
    let outcome: bool = test().run(code)?;

    assert!(outcome);
    Ok(())
}

#[test]
fn compound_comparison2() -> Result {
    let code = r#"
        4 < 3 or 2 > 1
    "#;
    let outcome: bool = test().run(code)?;

    assert!(outcome);
    Ok(())
}

#[test]
fn compound_where() -> Result {
    let code = r#"
        echo '[{"a": 1, "b": 1}, {"a": 2, "b": 1}, {"a": 2, "b": 2}]' | from json | where a == 2 and b == 1 | to json -r
    "#;
    let outcome: String = test().run(code)?;

    assert_eq!(outcome, r#"[{"a":2,"b":1}]"#);
    Ok(())
}

#[test]
fn compound_where_paren() -> Result {
    let code = r#"
        echo '[{"a": 1, "b": 1}, {"a": 2, "b": 1}, {"a": 2, "b": 2}]' | from json | where ($it.a == 2 and $it.b == 1) or $it.b == 2 | to json -r
    "#;
    let outcome: String = test().run(code)?;

    assert_eq!(outcome, r#"[{"a":2,"b":1},{"a":2,"b":2}]"#);
    Ok(())
}

// TODO: these ++ tests are not really testing *math* functionality, maybe find another place for them

#[test]
fn concat_lists() -> Result {
    let code = r#"
        [1 3] ++ [5 6] | to nuon
    "#;
    let outcome: String = test().run(code)?;

    assert_eq!(outcome, "[1, 3, 5, 6]");
    Ok(())
}

#[test]
fn concat_tables() -> Result {
    let code = r#"
        [[a b]; [1 2]] ++ [[c d]; [10 11]] | to nuon
    "#;
    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "[{a: 1, b: 2}, {c: 10, d: 11}]");
    Ok(())
}

#[test]
fn concat_strings() -> Result {
    let code = r#"
        "foo" ++ "bar"
    "#;
    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "foobar");
    Ok(())
}

#[test]
fn concat_binary_values() -> Result {
    let code = r#"
        0x[01 02] ++ 0x[03 04] | to nuon
    "#;
    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "0x[01020304]");
    Ok(())
}
