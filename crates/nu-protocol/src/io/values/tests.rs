use crate::io::ReadIterator;
use pretty_assertions::assert_eq;

use super::*;

#[test]
fn read_binary_passthrough() {
    let bins = vec![&[0, 1][..], &[2, 3][..]];
    let read = ReadIterator::new(bins.clone().into_iter());
    let iter = Values::new(read, Span::test_data(), None, ByteStreamType::Binary);

    let bins_values: Vec<Value> = bins
        .into_iter()
        .map(|bin| Value::binary(bin, Span::test_data()))
        .collect();
    assert_eq!(
        bins_values,
        iter.collect::<Result<Vec<Value>, _>>().expect("error")
    );
}

#[test]
fn read_string_clean() {
    let strs = vec!["Nushell", "が好きです"];
    let read = ReadIterator::new(strs.clone().into_iter());
    let iter = Values::new(read, Span::test_data(), None, ByteStreamType::String);

    let strs_values: Vec<Value> = strs
        .into_iter()
        .map(|string| Value::string(string, Span::test_data()))
        .collect();
    assert_eq!(
        strs_values,
        iter.collect::<Result<Vec<Value>, _>>().expect("error")
    );
}

#[test]
fn read_string_split_boundary() {
    let real = "Nushell最高!";
    let chunks = vec![&b"Nushell\xe6"[..], &b"\x9c\x80\xe9"[..], &b"\xab\x98!"[..]];
    let read = ReadIterator::new(chunks.into_iter());
    let iter = Values::new(read, Span::test_data(), None, ByteStreamType::String);

    let mut string = String::new();
    for value in iter {
        let chunk_string = value.expect("error").into_string().expect("not a string");
        string.push_str(&chunk_string);
    }
    assert_eq!(real, string);
}

#[test]
fn read_string_utf8_error() {
    let chunks = vec![&b"Nushell\xe6"[..], &b"\x9c\x80\xe9"[..], &b"\xab"[..]];
    let read = ReadIterator::new(chunks.into_iter());
    let iter = Values::new(read, Span::test_data(), None, ByteStreamType::String);

    let mut string = String::new();
    for value in iter {
        match value {
            Ok(value) => string.push_str(&value.into_string().expect("not a string")),
            Err(err) => {
                println!("string so far: {:?}", string);
                println!("got error: {err:?}");
                assert!(!string.is_empty());
                assert!(matches!(err, ShellError::NonUtf8Custom { .. }));
                return;
            }
        }
    }
    panic!("no error");
}

#[test]
fn read_unknown_fallback() {
    let chunks = vec![&b"Nushell"[..], &b"\x9c\x80\xe9abcd"[..], &b"efgh"[..]];
    let read = ReadIterator::new(chunks.into_iter());
    let mut iter = Values::new(read, Span::test_data(), None, ByteStreamType::Unknown);

    let mut get = || iter.next().expect("end of iter").expect("error");

    assert_eq!(Value::test_string("Nushell"), get());
    assert_eq!(Value::test_binary(b"\x9c\x80\xe9abcd"), get());
    // Once it's in binary mode it won't go back
    assert_eq!(Value::test_binary(b"efgh"), get());
}
