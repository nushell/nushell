use nu_test_support::prelude::*;

#[test]
fn par_each_does_not_flatten_nested_structures() -> Result {
    // This is a regression test for issue #8497
    let code = "[1 2 3] | par-each { |it| [$it, $it] } | sort";

    test().run(code).expect_value_eq([[1, 1], [2, 2], [3, 3]])
}

#[test]
fn par_each_streams_when_keep_order_is_not_set() -> Result {
    // If `par-each` tries to collect first, this would never finish.
    let code = "1.. | par-each --threads 1 {|x| $x } | first 3";

    test().run(code).expect_value_eq([1, 2, 3])
}

#[test]
fn par_each_streams_with_list_stream_input() -> Result {
    // `each` turns the input into a ListStream. If `par-each` collects first,
    // this pipeline would never finish on an unbounded input.
    let code = "1.. | each {|x| $x } | par-each --threads 1 {|x| $x } | first 3";

    test().run(code).expect_value_eq([1, 2, 3])
}

#[test]
fn par_each_keep_order_preserves_input_order() -> Result {
    let code = "[3 1 2] | par-each --threads 4 --keep-order {|x| $x }";

    test().run(code).expect_value_eq([3, 1, 2])
}
