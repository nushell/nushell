use nu_test_support::prelude::*;

#[test]
fn echo_range_is_lazy() -> Result {
    test()
        .run("echo 1..10000000000 | first 3")
        .expect_value_eq([1, 2, 3])
}

#[test]
fn echo_range_handles_inclusive() -> Result {
    test()
        .run("echo 1..3 | each { |x| $x }")
        .expect_value_eq([1, 2, 3])
}

#[test]
fn echo_range_handles_exclusive() -> Result {
    test()
        .run("echo 1..<3 | each { |x| $x }")
        .expect_value_eq([1, 2])
}

#[test]
fn echo_range_handles_inclusive_down() -> Result {
    test()
        .run("echo 3..1 | each { |it| $it }")
        .expect_value_eq([3, 2, 1])
}

#[test]
fn echo_range_handles_exclusive_down() -> Result {
    test()
        .run("echo 3..<1 | each { |it| $it }")
        .expect_value_eq([3, 2])
}

#[test]
fn echo_is_const() -> Result {
    test()
        .run("const val = echo 1..3; $val | take 10") // ensure the value is no longer a range
        .expect_value_eq([1, 2, 3])
}
