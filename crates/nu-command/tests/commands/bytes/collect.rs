use nu_test_support::prelude::*;

#[test]
fn test_stream() -> Result {
    let code = "
        [0x[01] 0x[02] 0x[03] 0x[04]]
        | filter {true}
        | bytes collect 0x[aa aa]
        | encode hex
    ";
    test().run(code).expect_value_eq("01AAAA02AAAA03AAAA04")
}

#[test]
fn test_stream_type() -> Result {
    let code = "
        [0x[01] 0x[02] 0x[03] 0x[04]]
        | filter {true}
        | bytes collect 0x[00]
        | describe -n
    ";
    test().run(code).expect_value_eq("binary (stream)")
}
