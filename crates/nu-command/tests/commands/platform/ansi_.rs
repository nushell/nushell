use nu_test_support::nu;

#[test]
fn test_ansi_shows_error_on_escape() {
    let actual = nu!(r"ansi --escape \");

    assert!(actual.err.contains("no need for escape characters"))
}

#[test]
fn test_ansi_list_outputs_table() {
    let actual = nu!("ansi --list | length");

    assert_eq!(actual.out, "440");
}

#[test]
fn test_ansi_codes() {
    let actual = nu!("$'(ansi clear_scrollback_buffer)'");
    assert_eq!(actual.out, "\x1b[3J");

    // Currently, bg is placed before fg in the results
    // It's okay if something internally changes this, but
    // if so, the test case will need to be updated to:
    // assert_eq!(actual.out, "\x1b[31;48;2;0;255;0mHello\x1b[0m");

    let actual = nu!("$'(ansi { fg: red, bg: \"#00ff00\" })Hello(ansi reset)'");
    assert_eq!(actual.out, "\x1b[48;2;0;255;0;31mHello\x1b[0m");
}
