//! Unit tests for [`crate::eval_scratch`].

use crate::eval_scratch::{Scratch, env_value, is_nu_interpolation};
use nu_protocol::{Span, Value};
use pretty_assertions::assert_eq;
use rstest::rstest;

fn sp() -> Span {
    Span::unknown()
}

/// One shadow variable, the shape `eval`/`interpolate` take.
fn vars(name: &str, value: Value) -> Vec<(String, Value)> {
    vec![(name.to_string(), value)]
}

#[test]
fn path_becomes_a_list_other_vars_stay_strings() {
    let joined =
        std::env::join_paths(["/one", "/two"].iter().map(std::path::Path::new)).expect("joinable");
    let value = env_value("PATH", joined.to_string_lossy().to_string());
    let list = value.as_list().expect("PATH is a list");
    assert_eq!(list.len(), 2);

    // Case-insensitive: Windows spells it `Path`.
    assert!(env_value("Path", "/one".to_string()).as_list().is_ok());

    let other = env_value("EDITOR", "hx".to_string());
    assert_eq!(other, Value::string("hx", sp()));
}

#[test]
fn eval_binds_shadow_variables() {
    let mut scratch = Scratch::new();
    let result = scratch
        .eval("$x + 1", &vars("x", Value::int(1, sp())))
        .expect("evaluates");
    assert_eq!(result, Value::int(2, sp()));
}

/// A parse error reaches the watch UI as a sentence. Rendering it with `Debug`
/// leaks the internal shape (`Span { start: .., end: .. }`) into the message.
#[test]
fn parse_errors_are_displayed_not_debugged() {
    let mut scratch = Scratch::new();
    let err = scratch
        .eval("$x +", &vars("x", Value::int(1, sp())))
        .expect_err("incomplete");
    assert!(
        !err.contains("Span {"),
        "message should not be a Debug dump: {err}"
    );
}

/// A watch expression can parse and still fail to compile. The message has to
/// be nushell's own, not the "missing compiled representation" that
/// `eval_block` reports for a block whose IR never got built.
#[test]
fn compile_errors_reach_the_watch_pane() {
    let mut scratch = Scratch::new();
    let err = scratch
        .eval("$env.PWD = \"/tmp\"", &[])
        .expect_err("cannot set PWD");

    assert!(
        err.contains("PWD cannot be set manually"),
        "unexpected message: {err}"
    );
}

/// The point of the `var_ids` / `blocks` caches: a logpoint on a hot line calls
/// `eval` once per hit, and every declaration or parse merges a delta into the
/// engine permanently. Re-evaluating the same expression must add nothing.
#[test]
fn repeat_evaluation_does_not_grow_the_engine() {
    let mut scratch = Scratch::new();
    let v = vars("x", Value::int(1, sp()));

    scratch.eval("$x + 1", &v).expect("evaluates");
    let before = scratch.engine_footprint();

    for _ in 0..5 {
        scratch.eval("$x + 1", &v).expect("evaluates");
    }

    assert_eq!(before, scratch.engine_footprint());
}

/// A failing expression must not leave a merged delta behind either — a broken
/// breakpoint condition is re-evaluated on every hit.
#[test]
fn repeat_failure_does_not_grow_the_engine() {
    let mut scratch = Scratch::new();
    let v = vars("x", Value::int(1, sp()));

    scratch.eval("$x +", &v).expect_err("incomplete");
    let before = scratch.engine_footprint();

    for _ in 0..5 {
        scratch.eval("$x +", &v).expect_err("incomplete");
    }

    assert_eq!(before, scratch.engine_footprint());
}

#[rstest]
#[case::double_quoted("$\"a ($x)\"", true)]
#[case::single_quoted("$'a'", true)]
#[case::bare_text("plain text", false)]
#[case::dap_braces_are_not_nu("iteration {x}", false)]
// Only the delimiters are checked, so `$"` alone is too short to qualify.
#[case::unterminated_opener("$\"", false)]
fn nu_interpolation_is_recognised_by_its_delimiters(#[case] input: &str, #[case] expected: bool) {
    assert_eq!(is_nu_interpolation(input), expected);
}

#[test]
fn interpolate_evaluates_a_whole_nu_literal() {
    let mut scratch = Scratch::new();
    let out = scratch.interpolate("$\"i is ($x)\"", &vars("x", Value::int(7, sp())));
    assert_eq!(out, "i is 7");
}

#[test]
fn interpolate_substitutes_dap_brace_segments() {
    let mut scratch = Scratch::new();
    let out = scratch.interpolate("i is {$x} now", &vars("x", Value::int(7, sp())));
    assert_eq!(out, "i is 7 now");
}

/// An unmatched brace is literal text, so a message that merely mentions `{`
/// still logs.
#[test]
fn interpolate_passes_unmatched_braces_through() {
    let mut scratch = Scratch::new();
    let out = scratch.interpolate("a { b", &[]);
    assert_eq!(out, "a { b");
}

/// A logpoint never pauses, so a failed segment has to be visible in the
/// message — swallowing it would leave no sign anything went wrong.
#[test]
fn interpolate_shows_a_placeholder_for_a_failed_segment() {
    let mut scratch = Scratch::new();
    let out = scratch.interpolate("value {$nope}", &[]);
    assert!(
        out.starts_with("value {error: "),
        "expected an error placeholder, got: {out}"
    );
}
