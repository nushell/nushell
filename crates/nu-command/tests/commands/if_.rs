use pretty_assertions::assert_matches;
use rstest::rstest;

use nu_test_support::prelude::*;

// Companion to the `for` non-block-body regression test
// (https://github.com/nushell/nushell/issues/13746): passing a non-block
// (e.g. a bare variable) as the `if` body must surface a clean
// `nu::compile::invalid_keyword_call` error from the IR compiler and never
// panic, per the AGENTS.md "No panicking on user input" guideline.
#[rstest]
#[case("if true $nu")]
#[case("if true $nu else { 1 }")]
fn if_with_non_block_body_errors_without_panic(#[case] src: &str) -> Result {
    let err = test().run(src).expect_compile_error()?;
    assert_matches!(err, CompileError::InvalidKeywordCall { .. });
    Ok(())
}

/// A non-bool condition is the same type mismatch in `if` and `while`.
#[rstest]
#[case::int("if 1 { }")]
#[case::string("if 'a' { } else { }")]
fn if_non_bool_condition(#[case] src: &str) -> Result {
    test()
        .run(src)
        .expect_error_code_eq("nu::shell::type_mismatch")
}

/// An `if` evaluated as a constant reports a non-bool condition the same way.
#[test]
fn const_if_non_bool_condition() -> Result {
    let err = test()
        .run("const x = if 1 { 2 } else { 3 }")
        .expect_parse_error()?
        .to_string();
    assert_contains("type_mismatch", err);
    Ok(())
}

/// A condition that fails while it runs reports its own error, not that it isn't a bool.
#[rstest]
#[case::if_("if ([1] | each { error make {msg: boom} }) { 1 }")]
#[case::while_("while ([1] | each { error make {msg: boom} }) { break }")]
fn failing_condition_reports_its_error(#[case] src: &str) -> Result {
    test()
        .run(format!("try {{ {src} }} catch {{|e| $e.msg }}"))
        .expect_value_eq("boom")
}
