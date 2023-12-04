use super::generic_digest::{GenericDigest, HashDigest};
use ::md5::Md5;
use nu_protocol::{Example, Span, Value};

pub type HashMd5 = GenericDigest<Md5>;

impl HashDigest for Md5 {
    fn name() -> &'static str {
        "md5"
    }

    fn examples() -> Vec<Example<'static>> {
        vec![
            Example {
                description: "Return the md5 hash of a string, hex-encoded",
                example: "'abcdefghijklmnopqrstuvwxyz' | hash md5",
                result: Some(Value::string(
                    "c3fcd3d76192e4007dfb496cca67e13b".to_owned(),
                    Span::test_data(),
                )),
            },
            Example {
                description: "Return the md5 hash of a string, as binary",
                example: "'abcdefghijklmnopqrstuvwxyz' | hash md5 --binary",
                result: Some(Value::binary(
                    vec![
                        0xc3, 0xfc, 0xd3, 0xd7, 0x61, 0x92, 0xe4, 0x00, 0x7d, 0xfb, 0x49, 0x6c,
                        0xca, 0x67, 0xe1, 0x3b,
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Return the md5 hash of a file's contents",
                example: "open ./nu_0_24_1_windows.zip | hash md5",
                result: None,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::generic_digest::{self, Arguments};

    #[test]
    fn test_examples() {
        crate::test_examples(HashMd5::default())
    }

    #[test]
    fn hash_string() {
        let binary = Value::string("abcdefghijklmnopqrstuvwxyz".to_owned(), Span::test_data());
        let expected = Value::string(
            "c3fcd3d76192e4007dfb496cca67e13b".to_owned(),
            Span::test_data(),
        );
        let actual = generic_digest::action::<Md5>(
            &binary,
            &Arguments {
                cell_paths: None,
                binary: false,
            },
            Span::test_data(),
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn hash_bytes() {
        let binary = Value::binary(vec![0xC0, 0xFF, 0xEE], Span::test_data());
        let expected = Value::string(
            "5f80e231382769b0102b1164cf722d83".to_owned(),
            Span::test_data(),
        );
        let actual = generic_digest::action::<Md5>(
            &binary,
            &Arguments {
                cell_paths: None,
                binary: false,
            },
            Span::test_data(),
        );
        assert_eq!(actual, expected);
    }
}
