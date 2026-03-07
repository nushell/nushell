use nu_test_support::prelude::*;

#[test]
fn generates_bytes_of_specified_length() -> Result {
    let code = "random binary 16 | bytes length";
    let outcome: i64 = test().run(code)?;
    assert_eq!(outcome, 16);
    Ok(())
}

#[test]
fn generates_bytes_of_specified_filesize() -> Result {
    let code = "random binary 1kb | bytes length";
    let outcome: i64 = test().run(code)?;
    assert_eq!(outcome, 1000);
    Ok(())
}
