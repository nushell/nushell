use super::generic_digest::{GenericDigest, HashDigest};
use nu_protocol::{Example, Span, Value};
use sha2::Sha512;

pub type HashSha512 = GenericDigest<Sha512>;

impl HashDigest for Sha512 {
    fn name() -> &'static str {
        "sha512"
    }

    fn examples() -> Vec<Example<'static>> {
        vec![
            Example {
                description: "Return the sha512 hash of a string, hex-encoded",
                example: "'abcdefghijklmnopqrstuvwxyz' | hash sha512",
                result: Some(Value::string(
                    "4dbff86cc2ca1bae1e16468a05cb9881c97f1753bce3619034898faa1aabe429955a1bf8ec483d7421fe3c1646613a59ed5441fb0f321389f77f48a879c7b1f1".to_owned(),
                    Span::test_data(),
                )),
            },
            Example {
                description: "Return the sha512 hash of a string, as binary",
                example: "'abcdefghijklmnopqrstuvwxyz' | hash sha512 --binary",
                result: Some(Value::binary(
                    vec![
                        0x4d, 0xbf, 0xf8, 0x6c, 0xc2, 0xca, 0x1b, 0xae, 0x1e, 0x16, 0x46, 0x8a,
                        0x05, 0xcb, 0x98, 0x81, 0xc9, 0x7f, 0x17, 0x53, 0xbc, 0xe3, 0x61, 0x90,
                        0x34, 0x89, 0x8f, 0xaa, 0x1a, 0xab, 0xe4, 0x29, 0x95, 0x5a, 0x1b, 0xf8,
                        0xec, 0x48, 0x3d, 0x74, 0x21, 0xfe, 0x3c, 0x16, 0x46, 0x61, 0x3a, 0x59,
                        0xed, 0x54, 0x41, 0xfb, 0x0f, 0x32, 0x13, 0x89, 0xf7, 0x7f, 0x48, 0xa8,
                        0x79, 0xc7, 0xb1, 0xf1],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Return the sha512 hash of binary data",
                example: "0x[deadbeef] | hash sha512",
                result: None,
            },
            Example {
                description: "Return the sha512 hash of a file's contents",
                example: "open ./nu_0_24_1_windows.zip | hash sha512",
                result: None,
            },
            Example {
                description: "Return the sha512 hash of a list of strings",
                example: "[abc def ghi] | hash sha512",
                result: Some(Value::list(
                    vec![
                        Value::string(
                            "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
                                .to_owned(),
                            Span::test_data(),
                        ),
                        Value::string(
                            "40a855bf0a93c1019d75dd5b59cd8157608811dd75c5977e07f3bc4be0cad98b22dde4db9ddb429fc2ad3cf9ca379fedf6c1dc4d4bb8829f10c2f0ee04a66663"
                                .to_owned(),
                            Span::test_data(),
                        ),
                        Value::string(
                            "366aead3bed29b6d1de2b8d211e791e5dc7a9611b3d4c61c9323128d746e670a69e9690ce5620efc3b36f6d1b655ce36a72a2fbed4927448b668f1e3f341c0d9"
                                .to_owned(),
                            Span::test_data(),
                        ),
                    ],
                    Span::test_data(),
                )),
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::generic_digest::{self, Arguments};

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(HashSha512::default())
    }

    #[test]
    fn hash_string() {
        let binary = Value::string("abcdefghijklmnopqrstuvwxyz".to_owned(), Span::test_data());
        let expected = Value::string(
            "4dbff86cc2ca1bae1e16468a05cb9881c97f1753bce3619034898faa1aabe429955a1bf8ec483d7421fe3c1646613a59ed5441fb0f321389f77f48a879c7b1f1".to_owned(),
            Span::test_data(),
        );
        let actual = generic_digest::action::<Sha512>(
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
            "d6f3d166b443b394f2505c48a5c6904c682d5a6fbe360d6c337a98f7ea6675f195157b33f599b600e39783c72024f91b4718651b4cfd08afcf6c06b9cdb6508c".to_owned(),
            Span::test_data(),
        );
        let actual = generic_digest::action::<Sha512>(
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
