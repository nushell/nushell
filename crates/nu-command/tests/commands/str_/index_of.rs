use nu_protocol::ShellError;
use nu_test_support::prelude::*;

#[test]
fn all_respects_range_start_index() -> Result {
    let code = "'a.b.c.d' | str index-of . --all --range 2.. | to nuon";
    test().run(code).expect_value_eq("[3, 5]")
}

#[test]
fn all_over_list_input() -> Result {
    let code = "['a.b' 'c'] | str index-of . --all | to nuon";
    test().run(code).expect_value_eq("[[1], []]")
}

#[test]
fn all_over_table_column() -> Result {
    let code = "[[a]; ['a.b'] ['c']] | str index-of . --all a | get a | to nuon";
    test().run(code).expect_value_eq("[[1], []]")
}

#[test]
fn all_grapheme_clusters_drops_mid_cluster_matches() -> Result {
    let mut tester = test();
    tester
        .run("'🇯🇵🇵' | str index-of -g '🇵' --all | to nuon")
        .expect_value_eq("[1]")?;
    tester
        .run("'🇯🇵🇯🇵' | str index-of -g '🇵' --all | to nuon")
        .expect_value_eq("[]")?;
    tester
        .run("'🇯🇵🇯🇵x' | str index-of -g '🇵' --all | to nuon")
        .expect_value_eq("[]")
}

#[test]
fn single_grapheme_clusters_skips_mid_cluster_match() -> Result {
    let code = "'🇯🇵🇵' | str index-of -g '🇵'";
    test().run(code).expect_value_eq(1)
}

#[test]
fn empty_pattern_is_rejected() -> Result {
    let code = "'abc' | str index-of ''";
    let err = test().run(code).expect_shell_error()?;
    match err {
        ShellError::TypeMismatch { err_message, .. } => {
            assert_eq!(err_message, "the pattern to find cannot be empty");
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[test]
fn empty_pattern_is_rejected_with_all() -> Result {
    let code = "'abc' | str index-of '' --all";
    let err = test().run(code).expect_shell_error()?;
    match err {
        ShellError::TypeMismatch { err_message, .. } => {
            assert_eq!(err_message, "the pattern to find cannot be empty");
            Ok(())
        }
        err => Err(err.into()),
    }
}
