use nu_test_support::nu;

// Companion to the `for` non-block-body regression test
// (https://github.com/nushell/nushell/issues/13746): passing a non-block
// (e.g. a bare variable) as the `if` body must surface a clean
// `nu::compile::invalid_keyword_call` error from the IR compiler and never
// panic, per the AGENTS.md "No panicking on user input" guideline.
#[test]
fn if_with_non_block_body_errors_without_panic() {
    for src in ["if true $nu", "if true $nu else { 1 }"] {
        let actual = nu!(src);
        assert!(actual.err.contains("invalid_keyword_call"));
        assert!(!actual.err.to_lowercase().contains("panic"));
    }
}
