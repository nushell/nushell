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
fn empty_pattern_matches_at_start() -> Result {
    let code = "'abc' | str index-of ''";
    test().run(code).expect_value_eq(0)
}

#[test]
fn empty_pattern_matches_at_end_with_end_flag() -> Result {
    let code = "'abc' | str index-of '' --end";
    test().run(code).expect_value_eq(3)
}

#[test]
fn empty_pattern_with_all_returns_every_boundary() -> Result {
    let code = "'abc' | str index-of '' --all | to nuon";
    test().run(code).expect_value_eq("[0, 1, 2, 3]")
}

#[test]
fn empty_pattern_with_all_and_grapheme_clusters_returns_every_grapheme_boundary() -> Result {
    let code = "'🇯🇵a' | str index-of '' --all --grapheme-clusters | to nuon";
    test().run(code).expect_value_eq("[0, 1, 2]")
}
