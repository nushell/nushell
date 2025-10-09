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

use nu_test_support::nu;

#[test]
fn one_arg() {
    let actual = nu!(r#"
        1
    "#);

    assert_eq!(actual.out, "1");
}

#[test]
fn add() {
    let actual = nu!(r#"
        1 + 1
    "#);

    assert_eq!(actual.out, "2");
}

#[test]
fn add_compound() {
    let actual = nu!(r#"
        1 + 2 + 2
    "#);

    assert_eq!(actual.out, "5");
}

#[test]
fn precedence_of_operators() {
    let actual = nu!(r#"
        1 + 2 * 2
    "#);

    assert_eq!(actual.out, "5");
}

#[test]
fn precedence_of_operators2() {
    let actual = nu!(r#"
        1 + 2 * 2 + 1
    "#);

    assert_eq!(actual.out, "6");
}

#[test]
fn precedence_of_operators3() {
    let actual = nu!(r#"
        5 - 5 * 10 + 5
    "#);

    assert_eq!(actual.out, "-40");
}

#[test]
fn precedence_of_operators4() {
    let actual = nu!(r#"
        5 - (5 * 10) + 5
    "#);

    assert_eq!(actual.out, "-40");
}

#[test]
fn division_of_ints() {
    let actual = nu!(r#"
        4 / 2
    "#);

    assert_eq!(actual.out, "2.0");
}

#[test]
fn division_of_ints2() {
    let actual = nu!(r#"
        1 / 4
    "#);

    assert_eq!(actual.out, "0.25");
}

#[test]
fn error_zero_division_int_int() {
    let actual = nu!(r#"
        1 / 0
    "#);

    assert!(actual.err.contains("division by zero"));
}

#[test]
fn error_zero_division_float_int() {
    let actual = nu!(r#"
        1.0 / 0
    "#);

    assert!(actual.err.contains("division by zero"));
}

#[test]
fn error_zero_division_int_float() {
    let actual = nu!(r#"
        1 / 0.0
    "#);

    assert!(actual.err.contains("division by zero"));
}

#[test]
fn error_zero_division_float_float() {
    let actual = nu!(r#"
        1.0 / 0.0
    "#);

    assert!(actual.err.contains("division by zero"));
}

#[test]
fn floor_division_of_ints() {
    let actual = nu!(r#"
        5 // 2
    "#);

    assert_eq!(actual.out, "2");
}

#[test]
fn floor_division_of_ints2() {
    let actual = nu!(r#"
        -3 // 2
    "#);

    assert_eq!(actual.out, "-2");
}

#[test]
fn floor_division_of_floats() {
    let actual = nu!(r#"
        -3.0 // 2.0
    "#);

    assert_eq!(actual.out, "-2.0");
}

#[test]
fn error_zero_floor_division_int_int() {
    let actual = nu!(r#"
        1 // 0
    "#);

    assert!(actual.err.contains("division by zero"));
}

#[test]
fn error_zero_floor_division_float_int() {
    let actual = nu!(r#"
        1.0 // 0
    "#);

    assert!(actual.err.contains("division by zero"));
}

#[test]
fn error_zero_floor_division_int_float() {
    let actual = nu!(r#"
        1 // 0.0
    "#);

    assert!(actual.err.contains("division by zero"));
}

#[test]
fn error_zero_floor_division_float_float() {
    let actual = nu!(r#"
        1.0 // 0.0
    "#);

    assert!(actual.err.contains("division by zero"));
}
#[test]
fn proper_precedence_history() {
    let actual = nu!(r#"
        2 / 2 / 2 + 1
    "#);

    assert_eq!(actual.out, "1.5");
}

#[test]
fn parens_precedence() {
    let actual = nu!(r#"
        4 * (6 - 3)
    "#);

    assert_eq!(actual.out, "12");
}

#[test]
fn modulo() {
    let actual = nu!(r#"
        9 mod 2
    "#);

    assert_eq!(actual.out, "1");
}

#[test]
fn floor_div_mod() {
    let actual = nu!("let q = 8 // -3; let r = 8 mod -3; 8 == $q * -3 + $r");
    assert_eq!(actual.out, "true");

    let actual = nu!("let q = -8 // 3; let r = -8 mod 3; -8 == $q * 3 + $r");
    assert_eq!(actual.out, "true");
}

#[test]
fn floor_div_mod_overflow() {
    let actual = nu!(format!("{} // -1", i64::MIN));
    assert!(actual.err.contains("overflow"));

    let actual = nu!(format!("{} mod -1", i64::MIN));
    assert!(actual.err.contains("overflow"));
}

#[test]
fn floor_div_mod_zero() {
    let actual = nu!("1 // 0");
    assert!(actual.err.contains("zero"));

    let actual = nu!("1 mod 0");
    assert!(actual.err.contains("zero"));
}

#[test]
fn floor_div_mod_large_num() {
    let actual = nu!(format!("{} // {}", i64::MAX, i64::MAX / 2));
    assert_eq!(actual.out, "2");

    let actual = nu!(format!("{} mod {}", i64::MAX, i64::MAX / 2));
    assert_eq!(actual.out, "1");
}

#[test]
fn unit_multiplication_math() {
    let actual = nu!("1MB * 2");
    assert_eq!(actual.out, "2.0 MB");
}

#[test]
fn unit_multiplication_float_math() {
    let actual = nu!("1MB * 1.2");
    assert_eq!(actual.out, "1.2 MB");
}

#[test]
fn unit_float_floor_division_math() {
    let actual = nu!("1MB // 3.0");
    assert_eq!(actual.out, "333.3 kB");
}

#[test]
fn unit_division_math() {
    let actual = nu!("1MB / 4");
    assert_eq!(actual.out, "250.0 kB");
}

#[test]
fn unit_float_division_math() {
    let actual = nu!("1MB / 3.2");
    assert_eq!(actual.out, "312.5 kB");
}

#[test]
fn duration_math() {
    let actual = nu!(r#"
        1wk + 1day
    "#);

    assert_eq!(actual.out, "1wk 1day");
}

#[test]
fn duration_decimal_math() {
    let actual = nu!(r#"
        5.5day + 0.5day
    "#);

    assert_eq!(actual.out, "6day");
}

#[test]
fn duration_math_with_nanoseconds() {
    let actual = nu!(r#"
        1wk + 10ns
    "#);

    assert_eq!(actual.out, "1wk 10ns");
}

#[test]
fn duration_decimal_math_with_nanoseconds() {
    let actual = nu!(r#"
        1.5wk + 10ns
    "#);

    assert_eq!(actual.out, "1wk 3day 12hr 10ns");
}

#[test]
fn duration_decimal_math_with_all_units() {
    let actual = nu!(r#"
        1wk + 3day + 8hr + 10min + 16sec + 121ms + 11us + 12ns
    "#);

    assert_eq!(actual.out, "1wk 3day 8hr 10min 16sec 121ms 11µs 12ns");
}

#[test]
fn duration_decimal_dans_test() {
    let actual = nu!(r#"
        3.14sec
    "#);

    assert_eq!(actual.out, "3sec 140ms");
}

#[test]
fn duration_math_with_negative() {
    let actual = nu!(r#"
        1day - 1wk
    "#);

    assert_eq!(actual.out, "-6day");
}

#[test]
fn compound_comparison() {
    let actual = nu!(r#"
        4 > 3 and 2 > 1
    "#);

    assert_eq!(actual.out, "true");
}

#[test]
fn compound_comparison2() {
    let actual = nu!(r#"
        4 < 3 or 2 > 1
    "#);

    assert_eq!(actual.out, "true");
}

#[test]
fn compound_where() {
    let actual = nu!(r#"
        echo '[{"a": 1, "b": 1}, {"a": 2, "b": 1}, {"a": 2, "b": 2}]' | from json | where a == 2 and b == 1 | to json -r
    "#);

    assert_eq!(actual.out, r#"[{"a":2,"b":1}]"#);
}

#[test]
fn compound_where_paren() {
    let actual = nu!(r#"
        echo '[{"a": 1, "b": 1}, {"a": 2, "b": 1}, {"a": 2, "b": 2}]' | from json | where ($it.a == 2 and $it.b == 1) or $it.b == 2 | to json -r
    "#);

    assert_eq!(actual.out, r#"[{"a":2,"b":1},{"a":2,"b":2}]"#);
}

// TODO: these ++ tests are not really testing *math* functionality, maybe find another place for them

#[test]
fn concat_lists() {
    let actual = nu!(r#"
        [1 3] ++ [5 6] | to nuon
    "#);

    assert_eq!(actual.out, "[1, 3, 5, 6]");
}

#[test]
fn concat_tables() {
    let actual = nu!(r#"
        [[a b]; [1 2]] ++ [[c d]; [10 11]] | to nuon
    "#);
    assert_eq!(actual.out, "[{a: 1, b: 2}, {c: 10, d: 11}]");
}

#[test]
fn concat_strings() {
    let actual = nu!(r#"
        "foo" ++ "bar"
    "#);
    assert_eq!(actual.out, "foobar");
}

#[test]
fn concat_binary_values() {
    let actual = nu!(r#"
        0x[01 02] ++ 0x[03 04] | to nuon
    "#);
    assert_eq!(actual.out, "0x[01020304]");
}
