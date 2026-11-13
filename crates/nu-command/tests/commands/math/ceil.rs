use nu_test_support::prelude::*;

#[test]
fn const_ceil() -> Result {
    test()
        .run("const CEIL = 1.5 | math ceil; $CEIL")
        .expect_value_eq(2)
}

#[test]
fn can_ceil_range_into_list() -> Result {
    let expected: Value = test().run("[2 3 4]")?;
    test()
        .run("(1.8)..(3.8) | math ceil")
        .expect_value_eq(expected)
}

#[test]
fn cannot_ceil_infinite_range() -> Result {
    let outcome = test().run("0.. | math ceil").expect_shell_error()?;

    assert!(matches!(outcome, ShellError::IncorrectValue { .. }));
    Ok(())
}

#[test]
fn filesize_ceil_is_noop_on_base_units() -> Result {
    // 2.1KB is stored as 2100 bytes; ceiling cannot use the display unit (KB).
    let expected: Value = test().run("2.1KB")?;
    test().run("2.1KB | math ceil").expect_value_eq(expected)
}
