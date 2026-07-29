use nu_test_support::prelude::*;

#[test]
fn convert_back_and_forth() -> Result {
    let code = "1 | into binary | into int";
    test().run(code).expect_value_eq(1)
}

#[test]
fn convert_into_int_little_endian() -> Result {
    let code = "0x[00 ff] | into int --endian little";
    test().run(code).expect_value_eq(65280)?;

    let code = "0x[01 00 00 00 00 00 00 00] | into int --endian little";
    test().run(code).expect_value_eq(1)?;

    let code = "0x[00 00 00 00 00 00 00 01] | into int --endian little";
    test().run(code).expect_value_eq(72057594037927936_i64)
}

#[test]
fn convert_into_int_big_endian() -> Result {
    let code = "0x[ff 00] | into int --endian big";
    test().run(code).expect_value_eq(65280)?;

    let code = "0x[00 00 00 00 00 00 00 01] | into int --endian big";
    test().run(code).expect_value_eq(1)?;

    let code = "0x[01 00 00 00 00 00 00 00] | into int --endian big";
    test().run(code).expect_value_eq(72057594037927936_i64)
}

#[test]
fn convert_into_int_at_unsigned_limit() -> Result {
    let code = "0x[ff ff ff ff ff ff ff 7f] | into int --endian little";
    test().run(code).expect_value_eq(i64::MAX)?;

    let code = "0x[7f ff ff ff ff ff ff ff] | into int --endian big";
    test().run(code).expect_value_eq(i64::MAX)
}

#[test]
fn convert_into_int_above_unsigned_limit() -> Result {
    for code in [
        "0x[00 00 00 00 00 00 00 80] | into int --endian little",
        "0x[80 00 00 00 00 00 00 00] | into int --endian big",
    ] {
        let err = test().run(code).expect_shell_error()?;
        match err {
            ShellError::IncorrectValue { msg, .. } => {
                assert_eq!(msg, "unsigned binary input is too large to convert to int");
            }
            err => return Err(err.into()),
        }
    }

    Ok(())
}

#[test]
fn convert_into_signed_int_little_endian() -> Result {
    let code = "0x[00 ff] | into int --endian little --signed";
    test().run(code).expect_value_eq(-256)?;

    let code = "0x[ff 00 00 00 00 00 00 00] | into int --endian little --signed";
    test().run(code).expect_value_eq(255)?;

    let code = "0x[00 00 00 00 00 00 00 ff] | into int --endian little --signed";
    test().run(code).expect_value_eq(-72057594037927936_i64)
}

#[test]
fn convert_into_signed_int_big_endian() -> Result {
    let code = "0x[ff 00] | into int --endian big --signed";
    test().run(code).expect_value_eq(-256)?;

    let code = "0x[00 00 00 00 00 00 00 ff] | into int --endian big --signed";
    test().run(code).expect_value_eq(255)?;

    let code = "0x[ff 00 00 00 00 00 00 00] | into int --endian big --signed";
    test().run(code).expect_value_eq(-72057594037927936_i64)
}
