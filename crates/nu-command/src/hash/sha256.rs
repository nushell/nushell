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
                description: "sha256 encode a string",
                example: "echo 'abcdefghijklmnopqrstuvwxyz' | hash sha256",
                result: Some(Value::String {
                    val: "71c480df93d6ae2f1efad1447c66c9525e316218cf51fc8d9ed832f2daf18b73"
                        .to_owned(),
                    span: Span::unknown(),
                }),
            },
            Example {
                description: "sha256 encode a file",
                example: "open ./nu_0_24_1_windows.zip | hash sha256",
                result: None,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::generic_digest;

    #[test]
    fn test_examples() {
        crate::test_examples(HashSha256::default())
    }

    #[test]
    fn hash_string() {
        let binary = Value::String {
            val: "abcdefghijklmnopqrstuvwxyz".to_owned(),
            span: Span::unknown(),
        };
        let expected = Value::String {
            val: "71c480df93d6ae2f1efad1447c66c9525e316218cf51fc8d9ed832f2daf18b73".to_owned(),
            span: Span::unknown(),
        };
        let actual = generic_digest::action::<Sha256>(&binary);
        assert_eq!(actual, expected);
    }

    #[test]
    fn hash_bytes() {
        let binary = Value::Binary {
            val: vec![0xC0, 0xFF, 0xEE],
            span: Span::unknown(),
        };
        let expected = Value::String {
            val: "c47a10dc272b1221f0380a2ae0f7d7fa830b3e378f2f5309bbf13f61ad211913".to_owned(),
            span: Span::unknown(),
        };
        let actual = generic_digest::action::<Sha256>(&binary);
        assert_eq!(actual, expected);
    }
}
