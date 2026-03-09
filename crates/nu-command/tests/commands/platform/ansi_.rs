use nu_test_support::prelude::*;

#[test]
fn test_ansi_shows_error_on_escape() -> Result {
    let err = test().run(r"ansi --escape \").expect_shell_error()?;
    match err {
        ShellError::TypeMismatch { err_message, .. }
            if err_message == "no need for escape characters" =>
        {
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[test]
fn test_ansi_list_outputs_table() -> Result {
    test().run("ansi --list | length").expect_value_eq(440)
}

#[test]
fn test_ansi_codes() -> Result {
    test()
        .run("$'(ansi clear_scrollback_buffer)'")
        .expect_value_eq("\x1b[3J")?;

    // Currently, bg is placed before fg in the results
    // It's okay if something internally changes this, but
    // if so, the test case will need to be updated to:
    // assert_eq!(actual.out, "\x1b[31;48;2;0;255;0mHello\x1b[0m");

    test()
        .run("$'(ansi { fg: red, bg: \"#00ff00\" })Hello(ansi reset)'")
        .expect_value_eq("\x1b[48;2;0;255;0;31mHello\x1b[0m")
}
