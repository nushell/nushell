use crate::prelude::*;
use md5::Md5;
use nu_protocol::UntaggedValue;

use super::generic_digest::{self, HashDigest};

pub type SubCommand = generic_digest::SubCommand<Md5>;

impl HashDigest for Md5 {
    fn name() -> &'static str {
        "md5"
    }

    fn examples() -> Vec<Example> {
        vec![
            Example {
                description: "md5 encode a string",
                example: "echo 'abcdefghijklmnopqrstuvwxyz' | hash md5",
                result: Some(vec![UntaggedValue::string(
                    "c3fcd3d76192e4007dfb496cca67e13b",
                )
                .into_untagged_value()]),
            },
            Example {
                description: "md5 encode a file",
                example: "open ./nu_0_24_1_windows.zip | hash md5",
                result: Some(vec![UntaggedValue::string(
                    "dcf30f2836a1a99fc55cf72e28272606",
                )
                .into_untagged_value()]),
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use md5::Md5;
    use nu_protocol::{Primitive, UntaggedValue};
    use nu_source::Tag;
    use nu_test_support::value::string;

    use crate::commands::generators::hash_::generic_digest::action;

    #[test]
    fn md5_encode_string() {
        let word = string("abcdefghijklmnopqrstuvwxyz");
        let expected =
            UntaggedValue::string("c3fcd3d76192e4007dfb496cca67e13b").into_untagged_value();

        let actual = action::<Md5>(&word, Tag::unknown()).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn md5_encode_bytes() {
        let bytes = vec![0xC0, 0xFF, 0xEE];
        let binary = UntaggedValue::Primitive(Primitive::Binary(bytes)).into_untagged_value();
        let expected =
            UntaggedValue::string("5f80e231382769b0102b1164cf722d83").into_untagged_value();

        let actual = action::<Md5>(&binary, Tag::unknown()).unwrap();
        assert_eq!(actual, expected);
    }
}
