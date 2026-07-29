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

/// Default pool path (no `--threads`): dedicated cached pool of default size.
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

/// Default pool with keep-order.
#[test]
fn par_each_default_pool_keep_order() -> Result {
    let code = "[5 4 3 2 1] | par-each --keep-order {|x| $x }";

    test().run(code).expect_value_eq([5, 4, 3, 2, 1])
}

/// Streaming `par-each` after a ListStream producer that also uses Rayon (`ls`).
///
/// Regression for #18566 comment: sharing Rayon's global pool between a producer
/// (`ls`/`glob`) and the streaming `par-each` path can hang. `par-each` must use a
/// dedicated pool so producer and consumer never starve each other.
#[test]
fn par_each_after_ls_stream_does_not_deadlock() -> Result {
    // Compare against a sequential path so the test is independent of cwd contents.
    let code = "
        let n = ls | length
        let m = ls | wrap name | par-each {} | length
        $n == $m
    ";

    test().run(code).expect_value_eq(true)
}

/// Same producer/consumer separation with an identity closure and `--threads`.
#[test]
fn par_each_after_ls_stream_with_threads() -> Result {
    let code = "
        let n = ls | length
        let m = ls | wrap name | par-each --threads 2 {} | length
        $n == $m
    ";

    test().run(code).expect_value_eq(true)
}

/// `each` produces a ListStream; streaming `par-each` must still complete.
#[test]
fn par_each_after_each_stream_matches_length() -> Result {
    let code = "
        let data = 0..199 | each {|x| $x}
        let n = $data | length
        let m = $data | each {|x| {name: $x}} | par-each {} | length
        $n == $m
    ";

    test().run(code).expect_value_eq(true)
}

/// dc-glob schedules work on Rayon's global pool; `par-each` must not share it.
///
/// Mirrors the reported hang: `glob … | wrap name | par-each {}`.
#[test]
#[exp(nu_experimental::DC_GLOB)]
fn par_each_after_dc_glob_stream_does_not_deadlock() -> Result {
    let code = "
        let n = glob '*' | length
        let m = glob '*' | wrap name | par-each {} | length
        $n == $m
    ";

    test().run(code).expect_value_eq(true)
}

/// Nested `par-each --threads` must not deadlock by sharing one cached pool.
///
/// Regression: reusing the same pool for outer + inner work caused every worker to
/// block on the stream channel while nested jobs waited for a free thread.
#[test]
fn par_each_nested_same_thread_count_does_not_deadlock() -> Result {
    // 4 outer × sum(1..4) = 4 × 10 = 40
    let code = "
        1..4
        | par-each --threads 2 {|outer|
            1..4 | par-each --threads 2 {|inner| $inner } | math sum
        }
        | math sum
    ";

    test().run(code).expect_value_eq(40)
}

/// Nested keep-order with the same `--threads` count (install path).
#[test]
fn par_each_nested_keep_order_same_threads() -> Result {
    let code = "
        1..4
        | par-each --threads 2 --keep-order {|outer|
            1..4 | par-each --threads 2 --keep-order {|inner| $inner * $outer } | math sum
        }
        | math sum
    ";
    // outer 1: 10, outer 2: 20, outer 3: 30, outer 4: 40 => 100
    test().run(code).expect_value_eq(100)
}

/// Nested default pools (shared outer would deadlock if nested reused it).
#[test]
fn par_each_nested_default_pool_does_not_deadlock() -> Result {
    let code = "
        1..4
        | par-each {|outer|
            1..4 | par-each {|inner| $inner } | math sum
        }
        | math sum
    ";

    test().run(code).expect_value_eq(40)
}

/// Three levels of nesting with a small explicit pool (stdlib test runner shape).
#[test]
fn par_each_triple_nested_threads() -> Result {
    let code = "
        1..3
        | par-each --threads 2 {|a|
            1..3
            | par-each --threads 2 {|b|
                1..3 | par-each --threads 2 {|c| $a + $b + $c } | math sum
            }
            | math sum
        }
        | math sum
    ";
    // For each (a,b): sum_c (a+b+c) for c in 1..3 = 3(a+b)+6
    // For each a: sum_b of that = 3*sum_b(a+b)+18 = 3*(3a+6)+18 = 9a+36
    // sum_a = 9*(1+2+3)+3*36 = 54+108 = 162
    test().run(code).expect_value_eq(162)
}

/// Nested pattern used by `testing.nu`: outer modules, inner tests, same thread count.
#[test]
fn par_each_nested_modules_and_tests_pattern() -> Result {
    let code = "
        let modules = [
            {name: m1, tests: [a b c d]}
            {name: m2, tests: [a b c d]}
            {name: m3, tests: [a b c d]}
            {name: m4, tests: [a b c d]}
        ]
        $modules
        | par-each --threads 2 {|module|
            $module.tests
            | par-each --threads 2 {|test|
                $'($module.name)/($test)'
            }
            | length
        }
        | math sum
    ";

    test().run(code).expect_value_eq(16)
}
