use crate::tests::{run_test, TestResult};

#[test]
fn better_block_types() -> TestResult {
    run_test(
        r#"([1, 2, 3] | each -n { |it| $"($it.index) is ($it.item)" }).1"#,
        "1 is 2",
    )
}

#[test]
fn row_iteration() -> TestResult {
    run_test(
        "[[name, size]; [tj, 100], [rl, 200]] | each { |it| $it.size * 8 } | get 1",
        "1600",
    )
}

#[test]
fn row_condition1() -> TestResult {
    run_test(
        "([[name, size]; [a, 1], [b, 2], [c, 3]] | where size < 3).name | get 1",
        "b",
    )
}

#[test]
fn row_condition2() -> TestResult {
    run_test(
        "[[name, size]; [a, 1], [b, 2], [c, 3]] | where $it.size > 2 | length",
        "1",
    )
}

#[test]
fn par_each() -> TestResult {
    run_test(
        r#"1..10 | par-each --numbered { |it| ([[index, item]; [$it.index, ($it.item > 5)]]).0 } | where index == 4 | get item.0"#,
        "false",
    )
}
