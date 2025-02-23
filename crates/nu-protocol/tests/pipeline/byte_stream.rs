use nu_protocol::{ast::RangeInclusion, ByteStream, IntRange, Signals, Span, Value};

#[test]
pub fn test_simple_positive_slice_exclusive() {
    let data = b"Hello World".to_vec();
    let stream = ByteStream::read_binary(data, Span::test_data(), Signals::empty());
    let sliced = stream
        .slice(
            Span::test_data(),
            Span::test_data(),
            create_range(0, 5, RangeInclusion::RightExclusive),
        )
        .unwrap();
    let result = sliced.into_bytes().unwrap();
    let result = String::from_utf8(result).unwrap();
    assert_eq!(result, "Hello");
}

#[test]
pub fn test_simple_positive_slice_exclusive_streaming() {
    let data = b"Hello World".to_vec();
    let stream = ByteStream::read_binary(data, Span::test_data(), Signals::empty());
    let sliced = stream
        .with_known_size(None)
        .slice(
            Span::test_data(),
            Span::test_data(),
            create_range(0, 5, RangeInclusion::RightExclusive),
        )
        .unwrap();
    let result = sliced.into_bytes().unwrap();
    let result = String::from_utf8(result).unwrap();
    assert_eq!(result, "Hello");
}

#[test]
pub fn test_negative_start_exclusive() {
    let data = b"Hello World".to_vec();
    let stream = ByteStream::read_binary(data, Span::test_data(), Signals::empty());
    let sliced = stream
        .slice(
            Span::test_data(),
            Span::test_data(),
            create_range(-5, 11, RangeInclusion::RightExclusive),
        )
        .unwrap();
    let result = sliced.into_bytes().unwrap();
    let result = String::from_utf8(result).unwrap();
    assert_eq!(result, "World");
}

#[test]
pub fn test_negative_end_exclusive() {
    let data = b"Hello World".to_vec();
    let stream = ByteStream::read_binary(data, Span::test_data(), Signals::empty());
    let sliced = stream
        .slice(
            Span::test_data(),
            Span::test_data(),
            create_range(0, -6, RangeInclusion::RightExclusive),
        )
        .unwrap();
    let result = sliced.into_bytes().unwrap();
    let result = String::from_utf8(result).unwrap();
    assert_eq!(result, "Hello");
}

#[test]
pub fn test_negative_start_and_end_exclusive() {
    let data = b"Hello World".to_vec();
    let stream = ByteStream::read_binary(data, Span::test_data(), Signals::empty());
    let sliced = stream
        .slice(
            Span::test_data(),
            Span::test_data(),
            create_range(-5, -2, RangeInclusion::RightExclusive),
        )
        .unwrap();
    let result = sliced.into_bytes().unwrap();
    let result = String::from_utf8(result).unwrap();
    assert_eq!(result, "Wor");
}

#[test]
pub fn test_empty_slice_exclusive() {
    let data = b"Hello World".to_vec();
    let stream = ByteStream::read_binary(data, Span::test_data(), Signals::empty());
    let sliced = stream
        .slice(
            Span::test_data(),
            Span::test_data(),
            create_range(5, 5, RangeInclusion::RightExclusive),
        )
        .unwrap();
    let result = sliced.into_bytes().unwrap();
    let result = String::from_utf8(result).unwrap();
    assert_eq!(result, "");
}

#[test]
pub fn test_out_of_bounds_exclusive() {
    let data = b"Hello World".to_vec();
    let stream = ByteStream::read_binary(data, Span::test_data(), Signals::empty());
    let sliced = stream
        .slice(
            Span::test_data(),
            Span::test_data(),
            create_range(0, 20, RangeInclusion::RightExclusive),
        )
        .unwrap();
    let result = sliced.into_bytes().unwrap();
    let result = String::from_utf8(result).unwrap();
    assert_eq!(result, "Hello World");
}

#[test]
pub fn test_invalid_range_exclusive() {
    let data = b"Hello World".to_vec();
    let stream = ByteStream::read_binary(data, Span::test_data(), Signals::empty());
    let sliced = stream
        .slice(
            Span::test_data(),
            Span::test_data(),
            create_range(11, 5, RangeInclusion::RightExclusive),
        )
        .unwrap();
    let result = sliced.into_bytes().unwrap();
    let result = String::from_utf8(result).unwrap();
    assert_eq!(result, "");
}

#[test]
pub fn test_max_end_exclusive() {
    let data = b"Hello World".to_vec();
    let stream = ByteStream::read_binary(data, Span::test_data(), Signals::empty());
    let sliced = stream
        .slice(
            Span::test_data(),
            Span::test_data(),
            create_range(6, i64::MAX, RangeInclusion::RightExclusive),
        )
        .unwrap();
    let result = sliced.into_bytes().unwrap();
    let result = String::from_utf8(result).unwrap();
    assert_eq!(result, "World");
}

#[test]
pub fn test_simple_positive_slice_inclusive() {
    let data = b"Hello World".to_vec();
    let stream = ByteStream::read_binary(data, Span::test_data(), Signals::empty());
    let sliced = stream
        .slice(
            Span::test_data(),
            Span::test_data(),
            create_range(0, 5, RangeInclusion::RightExclusive),
        )
        .unwrap();
    let result = sliced.into_bytes().unwrap();
    let result = String::from_utf8(result).unwrap();
    assert_eq!(result, "Hello");
}

#[test]
pub fn test_negative_start_inclusive() {
    let data = b"Hello World".to_vec();
    let stream = ByteStream::read_binary(data, Span::test_data(), Signals::empty());
    let sliced = stream
        .slice(
            Span::test_data(),
            Span::test_data(),
            create_range(-5, 11, RangeInclusion::Inclusive),
        )
        .unwrap();
    let result = sliced.into_bytes().unwrap();
    let result = String::from_utf8(result).unwrap();
    assert_eq!(result, "World");
}

#[test]
pub fn test_negative_end_inclusive() {
    let data = b"Hello World".to_vec();
    let stream = ByteStream::read_binary(data, Span::test_data(), Signals::empty());
    let sliced = stream
        .slice(
            Span::test_data(),
            Span::test_data(),
            create_range(0, -7, RangeInclusion::Inclusive),
        )
        .unwrap();
    let result = sliced.into_bytes().unwrap();
    let result = String::from_utf8(result).unwrap();
    assert_eq!(result, "Hello");
}

#[test]
pub fn test_negative_start_and_end_inclusive() {
    let data = b"Hello World".to_vec();
    let stream = ByteStream::read_binary(data, Span::test_data(), Signals::empty());
    let sliced = stream
        .slice(
            Span::test_data(),
            Span::test_data(),
            create_range(-5, -1, RangeInclusion::Inclusive),
        )
        .unwrap();
    let result = sliced.into_bytes().unwrap();
    let result = String::from_utf8(result).unwrap();
    assert_eq!(result, "World");
}

#[test]
pub fn test_empty_slice_inclusive() {
    let data = b"Hello World".to_vec();
    let stream = ByteStream::read_binary(data, Span::test_data(), Signals::empty());
    let sliced = stream
        .slice(
            Span::test_data(),
            Span::test_data(),
            create_range(5, 5, RangeInclusion::Inclusive),
        )
        .unwrap();
    let result = sliced.into_bytes().unwrap();
    let result = String::from_utf8(result).unwrap();
    assert_eq!(result, " ");
}

#[test]
pub fn test_out_of_bounds_inclusive() {
    let data = b"Hello World".to_vec();
    let stream = ByteStream::read_binary(data, Span::test_data(), Signals::empty());
    let sliced = stream
        .slice(
            Span::test_data(),
            Span::test_data(),
            create_range(0, 20, RangeInclusion::Inclusive),
        )
        .unwrap();
    let result = sliced.into_bytes().unwrap();
    let result = String::from_utf8(result).unwrap();
    assert_eq!(result, "Hello World");
}

#[test]
pub fn test_invalid_range_inclusive() {
    let data = b"Hello World".to_vec();
    let stream = ByteStream::read_binary(data, Span::test_data(), Signals::empty());
    let sliced = stream
        .slice(
            Span::test_data(),
            Span::test_data(),
            create_range(11, 5, RangeInclusion::Inclusive),
        )
        .unwrap();
    let result = sliced.into_bytes().unwrap();
    let result = String::from_utf8(result).unwrap();
    assert_eq!(result, "");
}

#[test]
pub fn test_max_end_inclusive() {
    let data = b"Hello World".to_vec();
    let stream = ByteStream::read_binary(data, Span::test_data(), Signals::empty());
    let sliced = stream
        .slice(
            Span::test_data(),
            Span::test_data(),
            create_range(6, i64::MAX, RangeInclusion::Inclusive),
        )
        .unwrap();
    let result = sliced.into_bytes().unwrap();
    let result = String::from_utf8(result).unwrap();
    assert_eq!(result, "World");
}

fn create_range(start: i64, end: i64, inclusion: RangeInclusion) -> IntRange {
    IntRange::new(
        Value::int(start, Span::unknown()),
        Value::nothing(Span::test_data()),
        Value::int(end, Span::unknown()),
        inclusion,
        Span::unknown(),
    )
    .unwrap()
}
