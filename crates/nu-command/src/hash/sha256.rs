use super::generic_digest::{GenericDigest, HashDigest};
use ::sha2::Sha256;
use nu_protocol::{Example, Span, Value};

pub type HashSha256 = GenericDigest<Sha256>;

impl HashDigest for Sha256 {
    fn name() -> &'static str {
        "sha256"
    }

    fn examples() -> Vec<Example> {
        vec![
            Example {
                description: "Return the sha256 hash of a string, hex-encoded",
                example: "echo 'abcdefghijklmnopqrstuvwxyz' | hash sha256",
                result: Some(Value::String {
                    val: "71c480df93d6ae2f1efad1447c66c9525e316218cf51fc8d9ed832f2daf18b73"
                        .to_owned(),
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Return the sha256 hash of a string, as binary",
                example: "echo 'abcdefghijklmnopqrstuvwxyz' | hash sha256 --binary",
                result: Some(Value::Binary {
                    val: vec![
                        0x71, 0xc4, 0x80, 0xdf, 0x93, 0xd6, 0xae, 0x2f, 0x1e, 0xfa, 0xd1, 0x44,
                        0x7c, 0x66, 0xc9, 0x52, 0x5e, 0x31, 0x62, 0x18, 0xcf, 0x51, 0xfc, 0x8d,
                        0x9e, 0xd8, 0x32, 0xf2, 0xda, 0xf1, 0x8b, 0x73,
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Return the sha256 hash of a file's contents",
                example: "open ./nu_0_24_1_windows.zip | hash sha256",
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
        crate::test_examples(HashSha256::default())
    }

    #[test]
    fn hash_string() {
        let binary = Value::String {
            val: "abcdefghijklmnopqrstuvwxyz".to_owned(),
            span: Span::test_data(),
        };
        let expected = Value::String {
            val: "71c480df93d6ae2f1efad1447c66c9525e316218cf51fc8d9ed832f2daf18b73".to_owned(),
            span: Span::test_data(),
        };
        let actual = generic_digest::action::<Sha256>(
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
        let binary = Value::Binary {
            val: vec![0xC0, 0xFF, 0xEE],
            span: Span::test_data(),
        };
        let expected = Value::String {
            val: "c47a10dc272b1221f0380a2ae0f7d7fa830b3e378f2f5309bbf13f61ad211913".to_owned(),
            span: Span::test_data(),
        };
        let actual = generic_digest::action::<Sha256>(
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
