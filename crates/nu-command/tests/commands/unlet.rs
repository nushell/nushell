use nu_protocol::{CompileError, ShellError};
use nu_test_support::prelude::*;
use pretty_assertions::assert_matches;
use rstest::rstest;

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

/// In a pipeline the parser replaces `$in` with a fresh variable named `$in`, which must still be
/// treated as the built-in.
#[test]
fn unlet_builtin_in() -> Result {
    let err = test().run("unlet $in").expect_compile_error()?;
    assert_matches!(
        err,
        CompileError::InvalidLiteral { msg, .. } if msg.contains("'$in' is a built-in variable")
    );
    Ok(())
}

/// `unlet` hides a constant for the rest of the block, including where the parser evaluates
/// constants, whichever overlay is active.
#[rstest]
#[case::read("const x = 1; unlet $x; $x")]
#[case::const_eval("const x = 1; unlet $x; const y = $x")]
#[case::in_pipeline("const x = 1; unlet $x | ignore; $x")]
#[case::alias("alias u = unlet; const x = 1; u $x; $x")]
#[case::inner_block("const x = 1; if true { unlet $x; $x }")]
#[case::outer_binding("const x = 1; do { const x = 2; unlet $x; $x }")]
#[case::cell_path_without_members("const x = 1; unlet $x.; $x")]
#[case::other_overlay("const x = 1; overlay new spam; unlet $x; overlay hide spam; $x")]
fn unlet_constant_hides_name(#[case] code: &str) -> Result {
    test()
        .run(code)
        .expect_error_code_eq("nu::parser::variable_not_found")
}

/// Like any other definition in a block, hiding the constant ends with the block, even one that
/// runs on the same stack (unlike `unlet` on a `let` variable, which is gone after the `if`).
#[test]
fn unlet_constant_in_inner_block() -> Result {
    test()
        .run("const x = 1; if true { unlet $x }; $x")
        .expect_value_eq(1)
}

/// A new variable with the name isn't hidden, even when the overlay that was active at the
/// `unlet` comes back, and importing the constant again brings it back.
#[rstest]
#[case::same_overlay("const x = 1; unlet $x; const x = 2; $x")]
#[case::other_overlay(
    "const x = 1; overlay new spam; unlet $x; overlay hide spam; const x = 2; overlay use spam; $x"
)]
#[case::import_again("module m { export const x = 2 }; use m x; unlet $x; use m x; $x")]
fn unlet_constant_then_redeclare(#[case] code: &str) -> Result {
    test().run(code).expect_value_eq(2)
}

/// `unlet` hides the variable its argument refers to, not whatever has that name where it's
/// called: an alias with the argument built in refers to the constant from its definition.
#[test]
fn unlet_constant_through_alias_with_argument() -> Result {
    test()
        .run("const x = 1; alias u = unlet $x; let x = 2; u; $x")
        .expect_value_eq(2)
}

/// `unlet --help` only shows the help, so the constant stays.
#[test]
fn unlet_constant_help() -> Result {
    test()
        .run("const x = 1; unlet --help $x | ignore; $x")
        .expect_value_eq(1)
}

/// A redirection target is evaluated before the command runs, so it can still use the constant
/// that the command unlets.
#[test]
fn unlet_constant_in_redirection_target() -> Result {
    Playground::setup("unlet_constant_in_redirection_target", |dirs, _sandbox| {
        let () = test()
            .cwd(dirs.test())
            .run("const f = 'out.txt'; unlet $f o> $f")?;
        assert!(dirs.test().join("out.txt").exists());
        Ok(())
    })
}

/// In a nested block, `scope variables` leaves the constant out after `unlet`, as the parser
/// does.
#[test]
fn unlet_constant_scope_variables() -> Result {
    test()
        .run("const x = 1; do { unlet $x; scope variables | where name == '$x' | is-empty }")
        .expect_value_eq(true)
}

/// What used the constant before `unlet` keeps its value: a completion list in a signature is
/// already evaluated, and a command body reads the constant when it runs.
#[test]
fn unlet_constant_after_use() -> Result {
    let mut tester = test();
    let code = "
        const variants = [foo bar baz]
        def my-command [arg: string@$variants] { $variants }
        unlet $variants
    ";
    let () = tester.run(code)?;

    tester
        .run("$variants")
        .expect_error_code_eq("nu::parser::variable_not_found")?;
    let completion = "
        scope commands
        | where name == my-command
        | get 0.signatures
        | values
        | first
        | where parameter_type == positional
        | get 0.completion
    ";
    tester
        .run(completion)
        .expect_value_eq(["foo", "bar", "baz"])?;
    tester
        .run("my-command foo")
        .expect_value_eq(["foo", "bar", "baz"])
}

/// A constant hidden at the top level is gone for later source units, like later REPL lines,
/// whichever overlay binds it.
#[rstest]
#[case::same_source(&["const x = 1; unlet $x", "$x"])]
#[case::earlier_source(&["const x = 1", "unlet $x", "$x"])]
#[case::other_overlay(&["const x = 1", "overlay new spam", "unlet $x", "$x"])]
#[case::other_overlay_same_source(&["const x = 1; overlay new spam; unlet $x", "$x"])]
fn unlet_constant_from_earlier_source(#[case] sources: &[&str]) -> Result {
    test()
        .run_multiple(sources)
        .expect_error_code_eq("nu::parser::variable_not_found")
}

/// An indirect call passes values rather than variables, so there is nothing to delete.
#[test]
fn unlet_indirect_call() -> Result {
    let err = test()
        .run("let x = 1; let name = 'unlet'; %($name) $x")
        .expect_shell_error()?;
    assert_matches!(
        err,
        ShellError::Generic(err) if err.error == "Can't call `unlet` indirectly"
    );
    Ok(())
}
