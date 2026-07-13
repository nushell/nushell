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

/// Default pool path (no `--threads`): global Rayon pool reuse.
#[test]
fn par_each_default_pool_works() -> Result {
    let code = "[1 2 3 4] | par-each {|x| $x * 2 } | sort";

    test().run(code).expect_value_eq([2, 4, 6, 8])
}

/// Many sequential `par-each` calls (pool create/cache must stay correct).
#[test]
fn par_each_many_sequential_calls() -> Result {
    let code = "
        mut total = 0
        for _ in 1..20 {
            let n = [1 2 3] | par-each {|x| $x } | math sum
            $total += $n
        }
        $total
    ";

    test().run(code).expect_value_eq(120) // 20 * (1+2+3)
}

/// Cached custom pool (`--threads`) still produces correct results across calls.
#[test]
fn par_each_threads_flag_repeated() -> Result {
    let code = "
        let a = [1 2 3] | par-each --threads 2 {|x| $x * 3 } | sort
        let b = [1 2 3] | par-each --threads 2 {|x| $x * 3 } | sort
        $a == $b and $a == [3 6 9]
    ";

    test().run(code).expect_value_eq(true)
}

/// Default pool with keep-order (no private pool install).
#[test]
fn par_each_default_pool_keep_order() -> Result {
    let code = "[5 4 3 2 1] | par-each --keep-order {|x| $x }";

    test().run(code).expect_value_eq([5, 4, 3, 2, 1])
}
