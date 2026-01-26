use crate::repl::tests::{TestResult, test_ide_contains};

#[test]
fn parser_recovers() -> TestResult {
    test_ide_contains(
        "3 + \"bob\"\nlet x = \"fred\"\n",
        &["--ide-check", "5"],
        "\"typename\":\"string\"",
    )
}
