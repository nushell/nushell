use nu_test_support::prelude::*;

#[test]
pub fn test_basic_non_const_success() -> Result {
    let code = "let a = 0x[00 00 00]; $a | bytes length";
    test().run(code).expect_value_eq(3)
}

#[test]
pub fn test_basic_const_success() -> Result {
    let code = "const a = 0x[00 00 00] | bytes length; $a";
    test().run(code).expect_value_eq(3)
}

#[test]
pub fn test_array_non_const_success() -> Result {
    let code = "let a = [0x[00 00 00] 0x[00]]; $a | bytes length | to nuon --raw";
    test().run(code).expect_value_eq("[3,1]")
}

#[test]
pub fn test_array_const_success() -> Result {
    let code = "const a = [0x[00 00 00] 0x[00]] | bytes length; $a | to nuon --raw";
    test().run(code).expect_value_eq("[3,1]")
}

#[test]
pub fn test_table_non_const_success() -> Result {
    let code = "let a = [[a]; [0x[00]] [0x[]] [0x[11 ff]]]; $a | bytes length a | to json --raw";
    test()
        .run(code)
        .expect_value_eq(r#"[{"a":1},{"a":0},{"a":2}]"#)
}

#[test]
pub fn test_table_const_success() -> Result {
    let code = "const a = [[a]; [0x[00]] [0x[]] [0x[11 ff]]] | bytes length a; $a | to json --raw";
    test()
        .run(code)
        .expect_value_eq(r#"[{"a":1},{"a":0},{"a":2}]"#)
}

#[test]
pub fn test_non_const_invalid_input() -> Result {
    let code = "let a = 0; $a | bytes length";
    let err = test().run(code).expect_parse_error()?;
    assert!(matches!(err, ParseError::InputMismatch { .. }));
    Ok(())
}

#[test]
pub fn test_const_invalid_input() -> Result {
    let code = "const a = 0 | bytes length";
    let err = test().run(code).expect_parse_error()?;
    assert!(matches!(err, ParseError::InputMismatch { .. }));
    Ok(())
}
