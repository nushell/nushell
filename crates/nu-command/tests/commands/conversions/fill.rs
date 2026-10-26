use nu_test_support::prelude::*;

#[test]
fn fill_valid_alignments() -> Result {
    test()
        .run("'nushell' | fill -w 10 -a l")
        .expect_value_eq("nushell   ")?;
    test()
        .run("'nushell' | fill -w 10 -a r")
        .expect_value_eq("   nushell")?;
    test()
        .run("'nushell' | fill -w 10 -a c")
        .expect_value_eq(" nushell  ")?;
    test()
        .run("'nushell' | fill -w 10 -a cr")
        .expect_value_eq("  nushell ")
}

#[test]
fn fill_invalid_alignment_errors() -> Result {
    let err = test()
        .run("'nushell' | fill -w 10 -a invalid")
        .expect_shell_error()?;
    match err {
        ShellError::InvalidValue { valid, actual, .. } => {
            assert!(valid.contains("left"));
            assert_eq!(actual, "invalid");
        }
        err => return Err(err.into()),
    }

    Ok(())
}
