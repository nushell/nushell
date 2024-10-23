use nu_test_support::nu;

#[test]
fn returns_error_for_relative_range_on_infinite_stream() {
    let actual = nu!("nu --testbin iecho 3 | bytes at ..-3");
    assert!(
        actual.err.contains(
            "Negative range values cannot be used with streams that don't specify a length"
        ),
        "Expected error message for negative range with infinite stream"
    );
}

#[test]
fn returns_bytes_for_fixed_range_on_infinite_stream() {
    let actual = nu!("nu --testbin iecho 3 | bytes at ..10 | decode");
    assert_eq!(
        actual.out, "33333",
        "Expected bytes from index 1 to 10, but got different output"
    );
}
