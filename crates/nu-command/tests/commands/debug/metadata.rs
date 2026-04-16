use nu_test_support::prelude::*;

/// When pipeline input is collected into null/nothing, `metadata <expr>` should still work.
/// Regression: https://github.com/nushell/nushell/issues/16600
#[test]
fn metadata_positional_with_null_pipeline_input() -> Result {
    let code = "null | metadata 42 | get span | describe";
    test().run(code).expect_value_eq("record<start: int, end: int>")
}

#[test]
fn metadata_positional_ignores_pipeline_input() -> Result {
    let code = "1 | metadata 2 | get span | describe";
    test().run(code).expect_value_eq("record<start: int, end: int>")
}
