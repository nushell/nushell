use nu_test_support::prelude::*;

#[test]
fn is_terminal_returns_false_when_piped() -> Result {
    test().run("is-terminal | to text").expect_value_eq("false")
}

#[test]
fn is_terminal_returns_false_when_collected() -> Result {
    test()
        .run("let x = (is-terminal); $x")
        .expect_value_eq(false)
}

#[test]
fn is_terminal_rejects_multiple_streams() -> Result {
    test()
        .run("is-terminal --stdin --stderr")
        .expect_shell_error()?;
    Ok(())
}

#[test]
fn is_terminal_accepts_stdin_flag() -> Result {
    let value = test().run("is-terminal --stdin")?;
    assert!(matches!(value, Value::Bool { .. }));
    Ok(())
}

#[test]
fn is_terminal_defaults_to_stdout() -> Result {
    let value = test().run("is-terminal")?;
    assert!(matches!(value, Value::Bool { .. }));
    Ok(())
}
