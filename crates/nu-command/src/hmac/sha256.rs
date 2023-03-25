use super::generic_digest::{GenericDigest, HmacDigest};
use ::sha2::Sha256;
use nu_protocol::{Example, Span, Value};

pub type HmacSha256 = GenericDigest<::hmac::Hmac<Sha256>>;

impl HmacDigest for ::hmac::Hmac<Sha256> {
    fn name() -> &'static str {
        "sha256"
    }

    fn examples() -> Vec<Example<'static>> {
        vec![
            Example {
                description: "Return the sha256 hmac of a string, hex-encoded",
                example: "'abcdefghijklmnopqrstuvwxyz' | hmac sha256 --key 'mysecretkey'",
                result: Some(Value::String {
                    val: "87935cc95c18048a2060d02dc4296271669eb63ea247129892fbd1919303edb9"
                        .to_owned(),
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Return the sha256 hmac of a string, as binary",
                example: "'abcdefghijklmnopqrstuvwxyz' | hmac sha256 --key 'mysecretkey' --binary",
                result: Some(Value::Binary {
                    val: vec![
                        0x87, 0x93, 0x5c, 0xc9, 0x5c, 0x18, 0x04, 0x8a, 0x20, 0x60, 0xd0, 0x2d,
                        0xc4, 0x29, 0x62, 0x71, 0x66, 0x9e, 0xb6, 0x3e, 0xa2, 0x47, 0x12, 0x98,
                        0x92, 0xfb, 0xd1, 0x91, 0x93, 0x03, 0xed, 0xb9,
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Return the sha256 hmac of a file's contents",
                example: "open ./nu_0_24_1_windows.zip | hmac sha256 --key 'mysecretkey'",
                result: None,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hmac::generic_digest::{self, Arguments};
    use digest::KeyInit;

    #[test]
    fn test_examples() {
        crate::test_examples(HmacSha256::default())
    }

    #[test]
    fn hmac_string() {
        let binary = Value::String {
            val: "abcdefghijklmnopqrstuvwxyz".to_owned(),
            span: Span::test_data(),
        };
        let key = "mysecretkey";
        let expected = Value::String {
            val: "87935cc95c18048a2060d02dc4296271669eb63ea247129892fbd1919303edb9".to_owned(),
            span: Span::test_data(),
        };
        let actual = generic_digest::action::<::hmac::Hmac<Sha256>>(
            &binary,
            &Arguments {
                cell_paths: None,
                binary: false,
                mac: ::hmac::Hmac::<Sha256>::new_from_slice(key.as_bytes()).unwrap(),
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
        let key = "mysecretkey";
        let expected = Value::String {
            val: "f7dc525fcdfafcb39459b7c83d29f302f952cd0a0d2c82cfa95031d965620b7c".to_owned(),
            span: Span::test_data(),
        };
        let actual = generic_digest::action::<::hmac::Hmac<Sha256>>(
            &binary,
            &Arguments {
                cell_paths: None,
                binary: false,
                mac: ::hmac::Hmac::<Sha256>::new_from_slice(key.as_bytes()).unwrap(),
            },
            Span::test_data(),
        );
        assert_eq!(actual, expected);
    }
}
