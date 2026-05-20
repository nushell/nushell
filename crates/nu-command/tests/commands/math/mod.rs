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
    let code = "
        1
    ";
    test().run(code).expect_value_eq(1)
}

#[test]
fn add() -> Result {
    let code = "
        1 + 1
    ";
    test().run(code).expect_value_eq(2)
}

#[test]
fn add_compound() -> Result {
    let code = "
        1 + 2 + 2
    ";
    test().run(code).expect_value_eq(5)
}

#[test]
fn precedence_of_operators() -> Result {
    let code = "
        1 + 2 * 2
    ";
    test().run(code).expect_value_eq(5)
}

#[test]
fn precedence_of_operators2() -> Result {
    let code = "
        1 + 2 * 2 + 1
    ";
    test().run(code).expect_value_eq(6)
}

#[test]
fn precedence_of_operators3() -> Result {
    let code = "
        5 - 5 * 10 + 5
    ";
    test().run(code).expect_value_eq(-40)
}

#[test]
fn precedence_of_operators4() -> Result {
    let code = "
        5 - (5 * 10) + 5
    ";
    test().run(code).expect_value_eq(-40)
}

#[test]
fn division_of_ints() -> Result {
    let code = "
        4 / 2
    ";
    test().run(code).expect_value_eq(2.0)
}

#[test]
fn division_of_ints2() -> Result {
    let code = "
        1 / 4
    ";
    test().run(code).expect_value_eq(0.25)
}

#[test]
fn error_zero_division_int_int() -> Result {
    let code = "
        1 / 0
    ";
    let outcome = test().run(code).expect_shell_error()?;

    assert!(matches!(outcome, ShellError::DivisionByZero { .. }));
    Ok(())
}

#[test]
fn error_zero_division_float_int() -> Result {
    let code = "
        1.0 / 0
    ";
    let outcome = test().run(code).expect_shell_error()?;

    assert!(matches!(outcome, ShellError::DivisionByZero { .. }));
    Ok(())
}

#[test]
fn error_zero_division_int_float() -> Result {
    let code = "
        1 / 0.0
    ";
    let outcome = test().run(code).expect_shell_error()?;

    assert!(matches!(outcome, ShellError::DivisionByZero { .. }));
    Ok(())
}

#[test]
fn error_zero_division_float_float() -> Result {
    let code = "
        1.0 / 0.0
    ";
    let outcome = test().run(code).expect_shell_error()?;

    assert!(matches!(outcome, ShellError::DivisionByZero { .. }));
    Ok(())
}

#[test]
fn floor_division_of_ints() -> Result {
    let code = "
        5 // 2
    ";
    test().run(code).expect_value_eq(2)
}

#[test]
fn floor_division_of_ints2() -> Result {
    let code = "
        -3 // 2
    ";
    test().run(code).expect_value_eq(-2)
}

#[test]
fn floor_division_of_floats() -> Result {
    let code = "
        -3.0 // 2.0
    ";
    test().run(code).expect_value_eq(-2.0)
}

#[test]
fn error_zero_floor_division_int_int() -> Result {
    let code = "
        1 // 0
    ";
    let outcome = test().run(code).expect_shell_error()?;

    assert!(matches!(outcome, ShellError::DivisionByZero { .. }));
    Ok(())
}

#[test]
fn error_zero_floor_division_float_int() -> Result {
    let code = "
        1.0 // 0
    ";
    let outcome = test().run(code).expect_shell_error()?;

    assert!(matches!(outcome, ShellError::DivisionByZero { .. }));
    Ok(())
}

#[test]
fn error_zero_floor_division_int_float() -> Result {
    let code = "
        1 // 0.0
    ";
    let outcome = test().run(code).expect_shell_error()?;

    assert!(matches!(outcome, ShellError::DivisionByZero { .. }));
    Ok(())
}

#[test]
fn error_zero_floor_division_float_float() -> Result {
    let code = "
        1.0 // 0.0
    ";
    let outcome = test().run(code).expect_shell_error()?;

    assert!(matches!(outcome, ShellError::DivisionByZero { .. }));
    Ok(())
}
#[test]
fn proper_precedence_history() -> Result {
    let code = "
        2 / 2 / 2 + 1
    ";
    test().run(code).expect_value_eq(1.5)
}

#[test]
fn parens_precedence() -> Result {
    let code = "
        4 * (6 - 3)
    ";
    test().run(code).expect_value_eq(12)
}

#[test]
fn modulo() -> Result {
    let code = "
        9 mod 2
    ";
    test().run(code).expect_value_eq(1)
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
    test().run(&code).expect_value_eq(2)?;

    let code = format!("{} mod {}", i64::MAX, i64::MAX / 2);
    test().run(&code).expect_value_eq(1)?;
    Ok(())
}

#[test]
fn unit_multiplication_math() -> Result {
    test()
        .run("1MB * 2")
        .expect_value_eq(Value::test_filesize(2_000_000))
}

#[test]
fn unit_multiplication_float_math() -> Result {
    test()
        .run("1MB * 1.2")
        .expect_value_eq(Value::test_filesize(1_200_000))
}

#[test]
fn unit_float_floor_division_math() -> Result {
    test()
        .run("1MB // 3.0")
        .expect_value_eq(Value::test_filesize(333_333))
}

#[test]
fn unit_division_math() -> Result {
    test()
        .run("1MB / 4")
        .expect_value_eq(Value::test_filesize(250_000))
}

#[test]
fn unit_float_division_math() -> Result {
    test()
        .run("1MB / 3.2")
        .expect_value_eq(Value::test_filesize(312_500))
}

#[test]
fn duration_math() -> Result {
    let code = "
        1wk + 1day
    ";
    test()
        .run(code)
        .expect_value_eq(Value::test_duration(691_200_000_000_000))
}

#[test]
fn duration_decimal_math() -> Result {
    let code = "
        5.5day + 0.5day
    ";
    test()
        .run(code)
        .expect_value_eq(Value::test_duration(518_400_000_000_000))
}

#[test]
fn duration_math_with_nanoseconds() -> Result {
    let code = "
        1wk + 10ns
    ";
    test()
        .run(code)
        .expect_value_eq(Value::test_duration(604_800_000_000_010))
}

#[test]
fn duration_decimal_math_with_nanoseconds() -> Result {
    let code = "
        1.5wk + 10ns
    ";
    test()
        .run(code)
        .expect_value_eq(Value::test_duration(907_200_000_000_010))
}

#[test]
fn duration_decimal_math_with_all_units() -> Result {
    let code = "
        1wk + 3day + 8hr + 10min + 16sec + 121ms + 11us + 12ns
    ";
    test()
        .run(code)
        .expect_value_eq(Value::test_duration(893_416_121_011_012))
}

#[test]
fn duration_decimal_dans_test() -> Result {
    let code = "
        3.14sec
    ";
    test()
        .run(code)
        .expect_value_eq(Value::test_duration(3_140_000_000))
}

#[test]
fn duration_math_with_negative() -> Result {
    let code = "
        1day - 1wk
    ";
    test()
        .run(code)
        .expect_value_eq(Value::test_duration(-518_400_000_000_000))
}

#[test]
fn compound_comparison() -> Result {
    let code = "
        4 > 3 and 2 > 1
    ";
    let outcome: bool = test().run(code)?;

    assert!(outcome);
    Ok(())
}

#[test]
fn compound_comparison2() -> Result {
    let code = "
        4 < 3 or 2 > 1
    ";
    let outcome: bool = test().run(code)?;

    assert!(outcome);
    Ok(())
}

#[test]
fn compound_where() -> Result {
    let code = r#"
        echo '[{"a": 1, "b": 1}, {"a": 2, "b": 1}, {"a": 2, "b": 2}]' | from json | where a == 2 and b == 1 | to json -r
    "#;
    test().run(code).expect_value_eq(r#"[{"a":2,"b":1}]"#)
}

#[test]
fn compound_where_paren() -> Result {
    let code = r#"
        echo '[{"a": 1, "b": 1}, {"a": 2, "b": 1}, {"a": 2, "b": 2}]' | from json | where ($it.a == 2 and $it.b == 1) or $it.b == 2 | to json -r
    "#;
    test()
        .run(code)
        .expect_value_eq(r#"[{"a":2,"b":1},{"a":2,"b":2}]"#)
}

// TODO: these ++ tests are not really testing *math* functionality, maybe find another place for them

#[test]
fn concat_lists() -> Result {
    let code = "
        [1 3] ++ [5 6] | to nuon
    ";
    test().run(code).expect_value_eq("[1, 3, 5, 6]")
}

#[test]
fn concat_tables() -> Result {
    let code = "
        [[a b]; [1 2]] ++ [[c d]; [10 11]] | to nuon
    ";
    test()
        .run(code)
        .expect_value_eq("[{a: 1, b: 2}, {c: 10, d: 11}]")
}

#[test]
fn concat_strings() -> Result {
    let code = r#"
        "foo" ++ "bar"
    "#;
    test().run(code).expect_value_eq("foobar")
}

#[test]
fn concat_binary_values() -> Result {
    let code = "
        0x[01 02] ++ 0x[03 04] | to nuon
    ";
    test().run(code).expect_value_eq("0x[01020304]")
}
