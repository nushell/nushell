use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::ShellTypeName;
use nu_protocol::{ColumnPath, Primitive, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tag;

pub struct SubCommand;

impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "hash md5"
    }

    fn signature(&self) -> Signature {
        Signature::build("hash md5").rest(
            SyntaxShape::ColumnPath,
            "optionally md5 encode data by column paths",
        )
    }

    fn usage(&self) -> &str {
        "md5 encode a value"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        operate(args)
    }

    fn examples(&self) -> Vec<Example> {
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

fn operate(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let column_paths: Vec<ColumnPath> = args.rest(0)?;

    Ok(args
        .input
        .map(move |v| {
            if column_paths.is_empty() {
                action(&v, v.tag())
            } else {
                let mut ret = v;

                for path in &column_paths {
                    ret = ret.swap_data_by_column_path(
                        path,
                        Box::new(move |old| action(old, old.tag())),
                    )?;
                }

                Ok(ret)
            }
        })
        .into_input_stream())
}

fn action(input: &Value, tag: impl Into<Tag>) -> Result<Value, ShellError> {
    match &input.value {
        UntaggedValue::Primitive(Primitive::String(s)) => {
            let md5_digest = md5::compute(s.as_bytes());
            Ok(UntaggedValue::string(&format!("{:x}", md5_digest)).into_value(tag))
        }
        UntaggedValue::Primitive(Primitive::Binary(bytes)) => {
            let md5_digest = md5::compute(bytes);
            Ok(UntaggedValue::string(&format!("{:x}", md5_digest)).into_value(tag))
        }
        other => {
            let got = format!("got {}", other.type_name());
            Err(ShellError::labeled_error(
                "value is not supported for hashing as md5",
                got,
                tag.into().span,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::action;
    use nu_protocol::{Primitive, UntaggedValue};
    use nu_source::Tag;
    use nu_test_support::value::string;

    #[test]
    fn md5_encode_string() {
        let word = string("abcdefghijklmnopqrstuvwxyz");
        let expected =
            UntaggedValue::string("c3fcd3d76192e4007dfb496cca67e13b").into_untagged_value();

        let actual = action(&word, Tag::unknown()).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn md5_encode_bytes() {
        let bytes = vec![0xC0, 0xFF, 0xEE];
        let binary = UntaggedValue::Primitive(Primitive::Binary(bytes)).into_untagged_value();
        let expected =
            UntaggedValue::string("5f80e231382769b0102b1164cf722d83").into_untagged_value();

        let actual = action(&binary, Tag::unknown()).unwrap();
        assert_eq!(actual, expected);
    }
}
