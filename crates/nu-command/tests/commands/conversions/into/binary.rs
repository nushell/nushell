use nu_test_support::prelude::*;

#[test]
fn sets_stream_from_internal_command_as_binary() -> Result {
    let code = "seq 1 10 | to text | into binary | describe";
    test().run(code).expect_value_eq("binary (stream)")
}

#[test]
#[deps(TESTBIN_COCOCO)]
fn sets_stream_from_external_command_as_binary() -> Result {
    let code = "^cococo | into binary | describe";
    test().run(code).expect_value_eq("binary (stream)")
}
