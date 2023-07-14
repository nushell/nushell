use nu_test_support::{nu, pipeline};

#[test]
fn par_each_does_not_flatten_nested_structures() {
    // This is a regression test for issue #8497
    let actual = nu!(r#"[1 2 3] | par-each { |it| [$it, $it] } | sort | to json --raw"#);

    assert_eq!(actual.out, "[[1,1],[2,2],[3,3]]");
}
