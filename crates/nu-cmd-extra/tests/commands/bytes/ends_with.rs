use nu_test_support::nu;

#[test]
fn basic_binary_end_with() {
    let actual = nu!(r#"
            "hello world" | into binary | bytes ends-with 0x[77 6f 72 6c 64]
        "#);

    assert_eq!(actual.out, "true");
}

#[test]
fn basic_string_fails() {
    let actual = nu!(r#"
            "hello world" | bytes ends-with 0x[77 6f 72 6c 64]
        "#);

    assert!(actual.err.contains("command doesn't support"));
    assert_eq!(actual.out, "");
}

#[test]
fn short_stream_binary() {
    let actual = nu!(r#"
            nu --testbin repeater (0x[01]) 5 | bytes ends-with 0x[010101]
        "#);

    assert_eq!(actual.out, "true");
}

#[test]
fn short_stream_mismatch() {
    let actual = nu!(r#"
            nu --testbin repeater (0x[010203]) 5 | bytes ends-with 0x[010204]
        "#);

    assert_eq!(actual.out, "false");
}

#[test]
fn short_stream_binary_overflow() {
    let actual = nu!(r#"
            nu --testbin repeater (0x[01]) 5 | bytes ends-with 0x[010101010101]
        "#);

    assert_eq!(actual.out, "false");
}

#[test]
fn long_stream_binary() {
    let actual = nu!(r#"
            nu --testbin repeater (0x[01]) 32768 | bytes ends-with 0x[010101]
        "#);

    assert_eq!(actual.out, "true");
}

#[test]
fn long_stream_binary_overflow() {
    // .. ranges are inclusive..inclusive, so we don't need to +1 to check for an overflow
    let actual = nu!(r#"
            nu --testbin repeater (0x[01]) 32768 | bytes ends-with (0..32768 | each {|| 0x[01] } | bytes collect)
        "#);

    assert_eq!(actual.out, "false");
}

#[test]
fn long_stream_binary_exact() {
    // ranges are inclusive..inclusive, so we don't need to +1 to check for an overflow
    let actual = nu!(r#"
            nu --testbin repeater (0x[01020304]) 8192 | bytes ends-with (0..<8192 | each {|| 0x[01020304] } | bytes collect)
        "#);

    assert_eq!(actual.out, "true");
}

#[test]
fn long_stream_string_exact() {
    // ranges are inclusive..inclusive, so we don't need to +1 to check for an overflow
    let actual = nu!(r#"
            nu --testbin repeater hell 8192 | bytes ends-with (0..<8192 | each {|| "hell" | into binary } | bytes collect)
        "#);

    assert_eq!(actual.out, "true");
}

#[test]
fn long_stream_mixed_exact() {
    // ranges are inclusive..inclusive, so we don't need to +1 to check for an overflow
    let actual = nu!(r#"
            let binseg = (0..<2048 | each {|| 0x[003d9fbf] } | bytes collect)
            let strseg = (0..<2048 | each {|| "hell" | into binary } | bytes collect)

            nu --testbin repeat_bytes 003d9fbf 2048 68656c6c 2048 | bytes ends-with (bytes build $binseg $strseg)
        "#);

    assert_eq!(
        actual.err, "",
        "invocation failed. command line limit likely reached"
    );
    assert_eq!(actual.out, "true");
}

#[test]
fn long_stream_mixed_overflow() {
    // ranges are inclusive..inclusive, so we don't need to +1 to check for an overflow
    let actual = nu!(r#"
            let binseg = (0..<2048 | each {|| 0x[003d9fbf] } | bytes collect)
            let strseg = (0..<2048 | each {|| "hell" | into binary } | bytes collect)

            nu --testbin repeat_bytes 003d9fbf 2048 68656c6c 2048 | bytes ends-with (bytes build 0x[01] $binseg $strseg)
        "#);

    assert_eq!(
        actual.err, "",
        "invocation failed. command line limit likely reached"
    );
    assert_eq!(actual.out, "false");
}
