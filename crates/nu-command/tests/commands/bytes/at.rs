use nu_test_support::prelude::*;

#[test]
pub fn returns_error_for_relative_range_on_infinite_stream() -> Result {
    let code = "nu --testbin iecho 3 | bytes at ..-3";
    let err = test().add_nu_to_path().run(code).expect_shell_error()?;
    assert!(matches!(
        err,
        ShellError::RelativeRangeOnInfiniteStream { .. }
    ));
    Ok(())
}

#[test]
pub fn returns_bytes_for_fixed_range_on_infinite_stream_including_end() -> Result {
    let code = "nu --testbin iecho 3 | bytes at ..10 | decode";
    test()
        .add_nu_to_path()
        .run(code)
        .expect_value_eq("3\n3\n3\n3\n3\n3")
}

#[test]
pub fn returns_bytes_for_fixed_range_on_infinite_stream_excluding_end() -> Result {
    let code = "nu --testbin iecho 3 | bytes at ..<9 | decode";
    test()
        .add_nu_to_path()
        .run(code)
        .expect_value_eq("3\n3\n3\n3\n3")
}

#[test]
pub fn test_string_returns_correct_slice_for_simple_positive_slice() -> Result {
    let code = "\"Hello World\" | encode utf8 | bytes at ..4 | decode";
    test().run(code).expect_value_eq("Hello")
}

#[test]
pub fn test_string_returns_correct_slice_for_negative_start() -> Result {
    let code = "\"Hello World\" | encode utf8 | bytes at (-5)..10 | decode";
    test().run(code).expect_value_eq("World")
}

#[test]
pub fn test_string_returns_correct_slice_for_negative_end() -> Result {
    let code = "\"Hello World\" | encode utf8 | bytes at ..-7 | decode";
    test().run(code).expect_value_eq("Hello")
}

#[test]
pub fn test_string_returns_correct_slice_for_empty_slice() -> Result {
    let code = "\"Hello World\" | encode utf8 | bytes at 5..<5 | decode";
    test().run(code).expect_value_eq("")
}

#[test]
pub fn test_string_returns_correct_slice_for_out_of_bounds() -> Result {
    let code = "\"Hello World\" | encode utf8 | bytes at ..20 | decode";
    test().run(code).expect_value_eq("Hello World")
}

#[test]
pub fn test_string_returns_correct_slice_for_invalid_range() -> Result {
    let code = "\"Hello World\" | encode utf8 | bytes at 11..5 | decode";
    test().run(code).expect_value_eq("")
}

#[test]
pub fn test_string_returns_correct_slice_for_max_end() -> Result {
    let code = "\"Hello World\" | encode utf8 | bytes at 6..<11 | decode";
    test().run(code).expect_value_eq("World")
}

#[test]
pub fn test_drops_content_type() -> Result {
    let code = format!(
        "open {} | bytes at 3..5 | metadata | get content_type? | describe",
        file!(),
    );
    test().run(code).expect_value_eq("nothing")
}
