use nu_test_support::prelude::*;

#[test]
fn basic_binary_starts_with() -> Result {
    test()
        .run(r#""hello world" | into binary | bytes starts-with 0x[68 65 6c 6c 6f]"#)
        .expect_value_eq(true)
}

#[test]
fn basic_string_fails() -> Result {
    test()
        .run(r#""hello world" | bytes starts-with 0x[68 65 6c 6c 6f]"#)
        .expect_error_code_eq("nu::parser::input_type_mismatch")
}

#[test]
#[deps(TESTBIN_REPEAT_BYTES)]
fn short_stream_binary() -> Result {
    test()
        .run("repeat_bytes 01 5 | bytes starts-with 0x[010101]")
        .expect_value_eq(true)
}

#[test]
#[deps(TESTBIN_REPEAT_BYTES)]
fn short_stream_mismatch() -> Result {
    test()
        .run("repeat_bytes 010203 5 | bytes starts-with 0x[010204]")
        .expect_value_eq(false)
}

#[test]
#[deps(TESTBIN_REPEAT_BYTES)]
fn short_stream_binary_overflow() -> Result {
    test()
        .run("repeat_bytes 01 5 | bytes starts-with 0x[010101010101]")
        .expect_value_eq(false)
}

#[test]
#[deps(TESTBIN_REPEAT_BYTES)]
fn long_stream_binary() -> Result {
    test()
        .run("repeat_bytes 01 32768 | bytes starts-with 0x[010101]")
        .expect_value_eq(true)
}

#[test]
#[deps(TESTBIN_REPEAT_BYTES)]
fn long_stream_binary_overflow() -> Result {
    // .. ranges are inclusive..inclusive, so we don't need to +1 to check for an overflow
    let code = "
        repeat_bytes 01 32768
        | bytes starts-with (0..32768 | each {|| 0x[01] } | bytes collect)
    ";

    test().run(code).expect_value_eq(false)
}

#[test]
#[deps(TESTBIN_REPEAT_BYTES)]
fn long_stream_binary_exact() -> Result {
    // ranges are inclusive..inclusive, so we don't need to +1 to check for an overflow
    let code = "
        repeat_bytes 01020304 8192
        | bytes starts-with (0..<8192 | each {|| 0x[01020304] } | bytes collect)
    ";

    test().run(code).expect_value_eq(true)
}

#[test]
#[deps(TESTBIN_REPEATER)]
fn long_stream_string_exact() -> Result {
    // ranges are inclusive..inclusive, so we don't need to +1 to check for an overflow
    let code = r#"
        repeater hell 8192
        | bytes starts-with (0..<8192 | each {|| "hell" | into binary } | bytes collect)
    "#;

    test().run(code).expect_value_eq(true)
}

#[test]
#[deps(TESTBIN_REPEAT_BYTES)]
fn long_stream_mixed_exact() -> Result {
    // ranges are inclusive..inclusive, so we don't need to +1 to check for an overflow
    let code = r#"
        let binseg = (0..<2048 | each {|| 0x[003d9fbf] } | bytes collect)
        let strseg = (0..<2048 | each {|| "hell" | into binary } | bytes collect)

        repeat_bytes 003d9fbf 2048 68656c6c 2048
        | bytes starts-with (bytes build $binseg $strseg)
    "#;

    test().run(code).expect_value_eq(true)
}

#[test]
#[deps(TESTBIN_REPEAT_BYTES)]
fn long_stream_mixed_overflow() -> Result {
    // ranges are inclusive..inclusive, so we don't need to +1 to check for an overflow
    let code = r#"
        let binseg = (0..<2048 | each {|| 0x[003d9fbf] } | bytes collect)
        let strseg = (0..<2048 | each {|| "hell" | into binary } | bytes collect)

        repeat_bytes 003d9fbf 2048 68656c6c 2048
        | bytes starts-with (bytes build $binseg $strseg 0x[01])
    "#;

    test().run(code).expect_value_eq(false)
}
