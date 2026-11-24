use crate::repl::tests::{TestResult, test_ide_contains};

#[test]
fn parser_recovers() -> TestResult {
    test_ide_contains(
        "3 + \"bob\"\nlet x = \"fred\"\n",
        &["--ide-check", "5"],
        "\"typename\":\"string\"",
    )
}

#[test]
fn parse_time_eval_error_message_is_one_line() -> TestResult {
    test_ide_contains(
        "hide $env.bar\n",
        &["--ide-check", "5"],
        "\"message\":\"Not a constant: Value is not a parse-time constant\"",
    )
}

/// The span is the evaluation error's own label, `into int`, not the whole value.
#[test]
fn parse_time_eval_error_points_at_the_cause() -> TestResult {
    test_ide_contains(
        "const d = ('abc' | into int)\n",
        &["--ide-check", "5"],
        "\"span\":{\"start\":19,\"end\":27}",
    )
}
