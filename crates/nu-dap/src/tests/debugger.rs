//! Unit tests for [`crate::debugger`].

use crate::debugger::exception_id;
use nu_protocol::shell_error::generic::GenericError;
use nu_protocol::{ShellError, Span};
use pretty_assertions::assert_eq;
use rstest::rstest;

/// The DAP `exceptionId` is nushell's diagnostic code, not anything scraped out
/// of `Debug`. Covers a variant with an explicit `code(..)` and a `transparent`
/// one that has to forward to its inner error.
#[rstest]
#[case::non_zero_exit_code(
    ShellError::NonZeroExitCode {
        exit_code: std::num::NonZeroI32::new(42).expect("nonzero"),
        span: Span::unknown(),
    },
    "nu::shell::non_zero_exit_code"
)]
#[case::division_by_zero(
    ShellError::DivisionByZero { span: Span::unknown() },
    "nu::shell::division_by_zero"
)]
#[case::generic_forwards_to_inner(
    ShellError::Generic(GenericError::new("boom", "it broke", Span::unknown())),
    "nu::shell::error"
)]
fn exception_id_is_the_diagnostic_code(#[case] err: ShellError, #[case] expected: &str) {
    assert_eq!(exception_id(&err), expected);
}

/// A custom code on a `GenericError` reaches the client as-is — the id is the
/// error's own identity, not a name we invent for it.
#[test]
fn exception_id_honours_a_custom_code() {
    let err = ShellError::Generic(
        GenericError::new("boom", "it broke", Span::unknown()).with_code("nu::dap::made_up"),
    );
    assert_eq!(exception_id(&err), "nu::dap::made_up");
}

/// `exceptionId` is a required DAP field, so it must never come back empty even
/// if a variant ever ships without a `code(..)`.
#[test]
fn exception_id_is_never_empty() {
    let err = ShellError::Generic(GenericError::new(
        "boom",
        "a message with spaces, braces { } and parens ( )",
        Span::unknown(),
    ));
    let id = exception_id(&err);
    assert!(!id.is_empty(), "empty id");
    // The message must not leak into the identifier.
    assert!(!id.contains(' '), "id carries payload: {id}");
}
