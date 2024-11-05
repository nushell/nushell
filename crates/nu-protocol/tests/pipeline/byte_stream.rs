use nu_protocol::{ByteStream, Signals, Span};

#[test]
pub fn test_simple_positive_slice() {
    let data = b"Hello World".to_vec();
    let stream = ByteStream::read_binary(data, Span::test_data(), Signals::empty());
    let sliced = stream
        .slice(Span::test_data(), Span::test_data(), 0, 5)
        .unwrap();
    let result = sliced.into_bytes().unwrap();
    assert_eq!(result, b"Hello");
}

#[test]
pub fn test_negative_start() {
    let data = b"Hello World".to_vec();
    let stream = ByteStream::read_binary(data, Span::test_data(), Signals::empty());
    let sliced = stream
        .slice(Span::test_data(), Span::test_data(), -5, 11)
        .unwrap();
    let result = sliced.into_bytes().unwrap();
    assert_eq!(result, b"World");
}

#[test]
pub fn test_negative_end() {
    let data = b"Hello World".to_vec();
    let stream = ByteStream::read_binary(data, Span::test_data(), Signals::empty());
    let sliced = stream
        .slice(Span::test_data(), Span::test_data(), 0, -6)
        .unwrap();
    let result = sliced.into_bytes().unwrap();
    assert_eq!(result, b"Hello");
}

#[test]
pub fn test_empty_slice() {
    let data = b"Hello World".to_vec();
    let stream = ByteStream::read_binary(data, Span::test_data(), Signals::empty());
    let sliced = stream
        .slice(Span::test_data(), Span::test_data(), 5, 5)
        .unwrap();
    let result = sliced.into_bytes().unwrap();
    assert_eq!(result, Vec::<u8>::new());
}

#[test]
pub fn test_out_of_bounds() {
    let data = b"Hello World".to_vec();
    let stream = ByteStream::read_binary(data, Span::test_data(), Signals::empty());
    let sliced = stream
        .slice(Span::test_data(), Span::test_data(), 0, 20)
        .unwrap();
    let result = sliced.into_bytes().unwrap();
    assert_eq!(result, b"Hello World");
}

#[test]
pub fn test_invalid_range() {
    let data = b"Hello World".to_vec();
    let stream = ByteStream::read_binary(data, Span::test_data(), Signals::empty());
    let sliced = stream
        .slice(Span::test_data(), Span::test_data(), 11, 5)
        .unwrap();
    let result = sliced.into_bytes().unwrap();
    assert_eq!(result, Vec::<u8>::new());
}

#[test]
pub fn test_max_end() {
    let data = b"Hello World".to_vec();
    let stream = ByteStream::read_binary(data, Span::test_data(), Signals::empty());
    let sliced = stream
        .slice(Span::test_data(), Span::test_data(), 6, isize::MAX)
        .unwrap();
    let result = sliced.into_bytes().unwrap();
    assert_eq!(result, b"World");
}
