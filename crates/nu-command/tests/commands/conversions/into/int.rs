use nu_test_support::prelude::*;

#[test]
fn convert_back_and_forth() -> Result {
    let code = r#"1 | into binary | into int"#;
    test().run(code).expect_value_eq(1)
}

#[test]
fn convert_into_int_little_endian() -> Result {
    let code = r#"0x[01 00 00 00 00 00 00 00] | into int --endian little"#;
    test().run(code).expect_value_eq(1)?;

    let code = r#"0x[00 00 00 00 00 00 00 01] | into int --endian little"#;
    test().run(code).expect_value_eq(72057594037927936_i64)
}

#[test]
fn convert_into_int_big_endian() -> Result {
    let code = r#"0x[00 00 00 00 00 00 00 01] | into int --endian big"#;
    test().run(code).expect_value_eq(1)?;

    let code = r#"0x[01 00 00 00 00 00 00 00] | into int --endian big"#;
    test().run(code).expect_value_eq(72057594037927936_i64)
}

#[test]
fn convert_into_signed_int_little_endian() -> Result {
    let code = r#"0x[ff 00 00 00 00 00 00 00] | into int --endian little --signed"#;
    test().run(code).expect_value_eq(255)?;

    let code = r#"0x[00 00 00 00 00 00 00 ff] | into int --endian little --signed"#;
    test().run(code).expect_value_eq(-72057594037927936_i64)
}

#[test]
fn convert_into_signed_int_big_endian() -> Result {
    let code = r#"0x[00 00 00 00 00 00 00 ff] | into int --endian big"#;
    test().run(code).expect_value_eq(255)?;

    let code = r#"0x[ff 00 00 00 00 00 00 00] | into int --endian big"#;
    test().run(code).expect_value_eq(-72057594037927936_i64)
}
