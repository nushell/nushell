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
