use nu_test_support::nu;

#[test]
fn window_size_negative() {
    let actual = nu!("[0 1 2] | window -1");
    assert!(actual.err.contains("positive"));
}

#[test]
fn window_size_zero() {
    let actual = nu!("[0 1 2] | window 0");
    assert!(actual.err.contains("zero"));
}

#[test]
fn window_size_not_int() {
    let actual = nu!("[0 1 2] | window (if true { 1sec })");
    assert!(actual.err.contains("can't convert"));
}

#[test]
fn stride_negative() {
    let actual = nu!("[0 1 2] | window 1 -s -1");
    assert!(actual.err.contains("positive"));
}

#[test]
fn stride_zero() {
    let actual = nu!("[0 1 2] | window 1 -s 0");
    assert!(actual.err.contains("zero"));
}

#[test]
fn stride_not_int() {
    let actual = nu!("[0 1 2] | window 1 -s (if true { 1sec })");
    assert!(actual.err.contains("can't convert"));
}

#[test]
fn empty() {
    let actual = nu!("[] | window 2 | is-empty");
    assert_eq!(actual.out, "true");
}

#[test]
fn list_stream() {
    let actual = nu!("([0 1 2] | every 1 | window 2) == ([0 1 2] | window 2)");
    assert_eq!(actual.out, "true");
}

#[test]
fn table_stream() {
    let actual = nu!("([[foo bar]; [0 1] [2 3] [4 5]] | every 1 | window 2) == ([[foo bar]; [0 1] [2 3] [4 5]] | window 2)");
    assert_eq!(actual.out, "true");
}

#[test]
fn no_empty_chunks() {
    let actual = nu!("([0 1 2 3 4 5] | window 3 -s 3 -r | length) == 2");
    assert_eq!(actual.out, "true");
}

#[test]
fn same_as_chunks() {
    let actual = nu!("([0 1 2 3 4] | window 2 -s 2 -r) == ([0 1 2 3 4 ] | chunks 2)");
    assert_eq!(actual.out, "true");
}

#[test]
fn stride_equal_to_window_size() {
    let actual = nu!("([0 1 2 3] | window 2 -s 2 | flatten) == [0 1 2 3]");
    assert_eq!(actual.out, "true");
}

#[test]
fn stride_greater_than_window_size() {
    let actual = nu!("([0 1 2 3 4] | window 2 -s 3 | flatten) == [0 1 3 4]");
    assert_eq!(actual.out, "true");
}

#[test]
fn stride_less_than_window_size() {
    let actual = nu!("([0 1 2 3 4 5] | window 3 -s 2 | length) == 2");
    assert_eq!(actual.out, "true");
}

#[test]
fn stride_equal_to_window_size_remainder() {
    let actual = nu!("([0 1 2 3 4] | window 2 -s 2 -r | flatten) == [0 1 2 3 4]");
    assert_eq!(actual.out, "true");
}

#[test]
fn stride_greater_than_window_size_remainder() {
    let actual = nu!("([0 1 2 3 4] | window 2 -s 3 -r | flatten) == [0 1 3 4]");
    assert_eq!(actual.out, "true");
}

#[test]
fn stride_less_than_window_size_remainder() {
    let actual = nu!("([0 1 2 3 4 5] | window 3 -s 2 -r | length) == 3");
    assert_eq!(actual.out, "true");
}
