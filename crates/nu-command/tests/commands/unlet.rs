use nu_protocol::{CompileError, ShellError};
use nu_test_support::prelude::*;
use pretty_assertions::assert_matches;

#[test]
fn unlet_basic() -> Result {
    let err = test()
        .run("let x = 42; unlet $x; $x")
        .expect_shell_error()?;
    assert_matches!(err, ShellError::VariableNotFoundAtRuntime { .. });
    Ok(())
}

#[test]
fn unlet_builtin_nu() -> Result {
    let err = test().run("unlet $nu").expect_compile_error()?;
    assert_matches!(
        err,
        CompileError::InvalidLiteral { msg, .. } if msg.contains("cannot be deleted")
    );
    Ok(())
}

#[test]
fn unlet_builtin_env() -> Result {
    let err = test().run("unlet $env").expect_compile_error()?;
    assert_matches!(
        err,
        CompileError::InvalidLiteral { msg, .. } if msg.contains("cannot be deleted")
    );
    Ok(())
}

#[test]
fn unlet_not_variable() -> Result {
    let err = test().run("unlet 42").expect_compile_error()?;
    assert_matches!(
        err,
        CompileError::InvalidLiteral { msg, .. }
            if msg == "Argument must be a variable reference like $x"
    );
    Ok(())
}

#[test]
fn unlet_wrong_number_args() -> Result {
    let err = test().run("unlet").expect_compile_error()?;
    assert_matches!(
        err,
        CompileError::InvalidLiteral { msg, .. } if msg == "unlet takes at least one argument"
    );
    Ok(())
}

#[test]
fn unlet_multiple_args() -> Result {
    let err = test()
        .run("let x = 1; let y = 2; unlet $x $y; $x")
        .expect_shell_error()?;
    assert_matches!(err, ShellError::VariableNotFoundAtRuntime { .. });
    Ok(())
}

#[test]
fn unlet_multiple_deletes_both() -> Result {
    let err = test()
        .run("let x = 1; let y = 2; unlet $x $y; $y")
        .expect_shell_error()?;
    assert_matches!(err, ShellError::VariableNotFoundAtRuntime { .. });
    Ok(())
}
