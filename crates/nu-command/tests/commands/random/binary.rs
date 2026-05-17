use nu_test_support::prelude::*;

#[test]
fn generates_bytes_of_specified_length() -> Result {
    let code = "random binary 16 | bytes length";
    test().run(code).expect_value_eq(16)
}

#[test]
fn generates_bytes_of_specified_filesize() -> Result {
    let code = "random binary 1kb | bytes length";
    test().run(code).expect_value_eq(1000)
}
