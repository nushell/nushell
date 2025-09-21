use nu_test_support::nu;

#[test]
pub fn returns_error_for_relative_range_on_infinite_stream() {
    let actual = nu!("nu --testbin iecho 3 | bytes at ..-3");
    assert!(
        actual.err.contains(
            "Relative range values cannot be used with streams that don't have a known length"
        ),
        "Expected error message for negative range with infinite stream"
    );
}

#[test]
pub fn returns_bytes_for_fixed_range_on_infinite_stream_including_end() {
    let actual = nu!("nu --testbin iecho 3 | bytes at ..10 | decode");
    assert_eq!(
        actual.out, "333333",
        "Expected bytes from index 0 to 10, but got different output"
    );
    let actual = nu!("nu --testbin iecho 3 | bytes at ..10 | decode");
    assert_eq!(
        actual.out, "333333",
        "Expected bytes from index 0 to 10, but got different output"
    );
}

#[test]
pub fn returns_bytes_for_fixed_range_on_infinite_stream_excluding_end() {
    let actual = nu!("nu --testbin iecho 3 | bytes at ..<9 | decode");
    assert_eq!(
        actual.out, "33333",
        "Expected bytes from index 0 to 8, but got different output"
    );
}

#[test]
pub fn test_string_returns_correct_slice_for_simple_positive_slice() {
    let actual = nu!("\"Hello World\" | encode utf8 | bytes at ..4 | decode");
    assert_eq!(actual.out, "Hello");
}

#[test]
pub fn test_string_returns_correct_slice_for_negative_start() {
    let actual = nu!("\"Hello World\" | encode utf8 | bytes at (-5)..10 | decode");
    assert_eq!(actual.out, "World");
}

#[test]
pub fn test_string_returns_correct_slice_for_negative_end() {
    let actual = nu!("\"Hello World\" | encode utf8 | bytes at ..-7 | decode");
    assert_eq!(actual.out, "Hello");
}

#[test]
pub fn test_string_returns_correct_slice_for_empty_slice() {
    let actual = nu!("\"Hello World\" | encode utf8 | bytes at 5..<5 | decode");
    assert_eq!(actual.out, "");
}

#[test]
pub fn test_string_returns_correct_slice_for_out_of_bounds() {
    let actual = nu!("\"Hello World\" | encode utf8 | bytes at ..20 | decode");
    assert_eq!(actual.out, "Hello World");
}

#[test]
pub fn test_string_returns_correct_slice_for_invalid_range() {
    let actual = nu!("\"Hello World\" | encode utf8 | bytes at 11..5 | decode");
    assert_eq!(actual.out, "");
}

#[test]
pub fn test_string_returns_correct_slice_for_max_end() {
    let actual = nu!("\"Hello World\" | encode utf8 | bytes at 6..<11 | decode");
    assert_eq!(actual.out, "World");
}

#[test]
pub fn test_drops_content_type() {
    let actual = nu!(format!(
        "open {} | bytes at 3..5 | metadata | get content_type? | describe",
        file!(),
    ));
    assert_eq!(actual.out, "nothing", "Expected content_type to be dropped");
}
