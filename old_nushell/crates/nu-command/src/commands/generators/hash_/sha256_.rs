use crate::prelude::*;
use nu_protocol::UntaggedValue;
use sha2::Sha256;

use super::generic_digest::{self, HashDigest};

pub type SubCommand = generic_digest::SubCommand<Sha256>;

impl HashDigest for Sha256 {
    fn name() -> &'static str {
        "sha256"
    }

    fn examples() -> Vec<Example> {
        vec![
            Example {
                description: "sha256 encode a string",
                example: "echo 'abcdefghijklmnopqrstuvwxyz' | hash sha256",
                result: Some(vec![UntaggedValue::string(
                    "71c480df93d6ae2f1efad1447c66c9525e316218cf51fc8d9ed832f2daf18b73",
                )
                .into_untagged_value()]),
            },
            Example {
                description: "sha256 encode a file",
                example: "open ./nu_0_24_1_windows.zip | hash sha256",
                result: Some(vec![UntaggedValue::string(
                    "c47a10dc272b1221f0380a2ae0f7d7fa830b3e378f2f5309bbf13f61ad211913",
                )
                .into_untagged_value()]),
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use nu_protocol::{Primitive, UntaggedValue};
    use nu_source::Tag;
    use nu_test_support::value::string;
    use sha2::Sha256;

    use crate::commands::generators::hash_::generic_digest::action;

    #[test]
    fn md5_encode_string() {
        let word = string("abcdefghijklmnopqrstuvwxyz");
        let expected = UntaggedValue::string(
            "71c480df93d6ae2f1efad1447c66c9525e316218cf51fc8d9ed832f2daf18b73",
        )
        .into_untagged_value();

        let actual = action::<Sha256>(&word, Tag::unknown()).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn md5_encode_bytes() {
        let bytes = vec![0xC0, 0xFF, 0xEE];
        let binary = UntaggedValue::Primitive(Primitive::Binary(bytes)).into_untagged_value();
        let expected = UntaggedValue::string(
            "c47a10dc272b1221f0380a2ae0f7d7fa830b3e378f2f5309bbf13f61ad211913",
        )
        .into_untagged_value();

        let actual = action::<Sha256>(&binary, Tag::unknown()).unwrap();
        assert_eq!(actual, expected);
    }
}
