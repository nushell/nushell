use nu_test_support::prelude::*;

#[test]
fn par_each_does_not_flatten_nested_structures() -> Result {
    // This is a regression test for issue #8497
    let code = "[1 2 3] | par-each { |it| [$it, $it] } | sort";

    test().run(code).expect_value_eq([[1, 1], [2, 2], [3, 3]])
}
