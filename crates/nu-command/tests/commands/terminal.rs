//! Tests for `is-terminal` (OS isatty) and `is-redirected` (Nu pipeline destination).
//!
//! Integration cases use `NuTester` (always `Stack::collect_value()`), so bare custom
//! command calls are treated as redirected. Unit cases below pin invocation-frame
//! semantics without going through the full evaluator.

use nu_command::IsRedirected;
use nu_protocol::{
    OutDest, PipelineData, Span, Value,
    engine::{Call, Command, EngineState, Stack},
};
use nu_test_support::prelude::*;
use rstest::rstest;

#[test]
fn is_terminal_rejects_multiple_streams() -> Result {
    test()
        .run("is-terminal --stdin --stderr")
        .expect_shell_error()?;
    Ok(())
}

#[test]
fn is_terminal_accepts_stdin_flag() -> Result {
    let _: bool = test().run("is-terminal --stdin")?;
    Ok(())
}

#[test]
fn is_terminal_defaults_to_stdout() -> Result {
    let _: bool = test().run("is-terminal")?;
    Ok(())
}

#[test]
fn is_terminal_inside_if_is_os_check_not_value_dest() -> Result {
    // Regression: `if (is-terminal)` must not force false via OutDest::Value.
    // Result must match a bare OS isatty check under the same process stdio.
    let bare: bool = test().run("is-terminal")?;
    let in_if: String =
        test().run(r#"if (is-terminal --stdout) { "terminal" } else { "piped" }"#)?;
    let expected = if bare { "terminal" } else { "piped" };
    assert_eq!(in_if, expected);
    Ok(())
}

#[test]
fn is_redirected_true_when_custom_command_piped() -> Result {
    test()
        .run("def pipetest [] { is-redirected }; pipetest | $in")
        .expect_value_eq(true)
}

#[test]
fn is_redirected_true_when_custom_command_collected() -> Result {
    test()
        .run("def pipetest [] { is-redirected }; let x = (pipetest); $x")
        .expect_value_eq(true)
}

#[test]
fn is_redirected_works_inside_if() -> Result {
    // Harness collect_value makes the call redirected; the assertion is that
    // `if (is-redirected)` sees the *call* frame, not the subexpression's Value dest.
    let code = r#"
        def pipetest [] {
            if (is-redirected) { "piped" } else { "display" }
        }

        pipetest
    "#;

    test().run(code).expect_value_eq("piped")
}

#[test]
fn is_redirected_true_when_piped_with_if() -> Result {
    let code = r#"
        def pipetest [] {
            if (is-redirected) { "piped" } else { "display" }
        }

        pipetest | $in
    "#;

    test().run(code).expect_value_eq("piped")
}

#[test]
fn is_redirected_nested_inner_sees_own_destination() -> Result {
    // `inner` is collected by `let`, so its own frame is redirected.
    let code = "
        def inner [] { is-redirected }
        def outer [] { let x = (inner); $x }
        outer
    ";

    test().run(code).expect_value_eq(true)
}

/// Run the `is-redirected` builtin against a prepared stack.
fn run_is_redirected(stack: &mut Stack) -> bool {
    let engine_state = EngineState::new();
    let call = Call::new(Span::test_data());
    let result = IsRedirected
        .run(&engine_state, stack, &call, PipelineData::empty())
        .expect("is-redirected should succeed");
    match result.into_value(Span::test_data()).expect("bool value") {
        Value::Bool { val, .. } => val,
        other => panic!("expected bool, got {other:?}"),
    }
}

#[rstest]
#[case::print(OutDest::Print, true, false)]
#[case::pipe(OutDest::Pipe, true, true)]
#[case::value(OutDest::Value, false, true)]
#[case::null(OutDest::Null, false, true)]
#[case::inherit(OutDest::Inherit, false, true)]
fn is_redirected_respects_invocation_frame(
    #[case] dest: OutDest,
    #[case] collect: bool,
    #[case] expected: bool,
) {
    // (invocation destination, simulate if-subexpression collect?, expected)
    let mut owned = Stack::new().with_invocation_stdout(dest);
    if collect {
        // Simulate `if (is-redirected)`: pipe_stdout becomes Value, but the
        // invocation frame must still control the answer.
        let mut collected = owned.start_collect_value();
        assert_eq!(collected.is_stdout_redirected(), expected);
        assert_eq!(run_is_redirected(&mut collected), expected);
    } else {
        assert_eq!(owned.is_stdout_redirected(), expected);
        assert_eq!(run_is_redirected(&mut owned), expected);
    }
}

#[test]
fn is_stdout_redirected_falls_back_to_current_stdout() {
    // Outside any custom command, use the stack's current stdout destination.
    let mut stack = Stack::new();
    assert!(!stack.is_stdout_redirected()); // default pipe_stdout is Print

    let stack = stack.start_collect_value();
    assert!(stack.is_stdout_redirected()); // Value
}
