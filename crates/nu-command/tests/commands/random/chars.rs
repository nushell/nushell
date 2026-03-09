use nu_test_support::prelude::*;

#[test]
fn generates_chars_of_specified_length() -> Result {
    let code = "random chars --length 15 | str stats | get chars";
    test().run(code).expect_value_eq(15)
}

#[test]
fn generates_chars_of_specified_filesize() -> Result {
    let code = "random chars --length 1kb | str stats | get bytes";
    test().run(code).expect_value_eq(1000)
}
