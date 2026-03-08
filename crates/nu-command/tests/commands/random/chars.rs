use nu_test_support::prelude::*;

#[test]
fn generates_chars_of_specified_length() -> Result {
    let code = "random chars --length 15 | str stats | get chars";
    let outcome: i64 = test().run(code)?;
    assert_eq!(outcome, 15);
    Ok(())
}

#[test]
fn generates_chars_of_specified_filesize() -> Result {
    let code = "random chars --length 1kb | str stats | get bytes";
    let outcome: i64 = test().run(code)?;
    assert_eq!(outcome, 1000);
    Ok(())
}
