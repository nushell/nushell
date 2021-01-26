use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::ShellTypeName;
use nu_protocol::{
    ColumnPath, Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};
use nu_source::{Tag, Tagged};
use nu_value_ext::ValueExt;

#[derive(Deserialize)]
struct Arguments {
    length: Tagged<usize>,
    character: Tagged<String>,
    rest: Vec<ColumnPath>,
}

pub struct SubCommand;

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "str lpad"
    }

    fn signature(&self) -> Signature {
        Signature::build("str lpad")
            .required_named("length", SyntaxShape::Int, "length to pad to", Some('l'))
            .required_named(
                "character",
                SyntaxShape::String,
                "character to pad with",
                Some('c'),
            )
            .rest(
                SyntaxShape::ColumnPath,
                "optionally check if string contains pattern by column paths",
            )
    }

    fn usage(&self) -> &str {
        "pad a string with a character a certain length"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        operate(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Left pad a string with a character a number of places",
                example: "echo 'nushell' | str lpad -l 10 -c '*'",
                result: Some(vec![
                    UntaggedValue::string("***nushell").into_untagged_value()
                ]),
            },
            Example {
                description: "Left pad a string with a character a number of places",
                example: "echo '123' | str lpad -l 10 -c '0'",
                result: Some(vec![
                    UntaggedValue::string("0000000123").into_untagged_value()
                ]),
            },
            Example {
                description: "Use lpad to truncate a string",
                example: "echo '123456789' | str lpad -l 3 -c '0'",
                result: Some(vec![UntaggedValue::string("123").into_untagged_value()]),
            },
            Example {
                description: "Use lpad to pad unicode",
                example: "echo '▉' | str lpad -l 10 -c '▉'",
                result: Some(vec![
                    UntaggedValue::string("▉▉▉▉▉▉▉▉▉▉").into_untagged_value()
                ]),
            },
        ]
    }
}

async fn operate(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let (
        Arguments {
            length,
            character,
            rest,
        },
        input,
    ) = args.process().await?;
    let column_paths: Vec<_> = rest;

    Ok(input
        .map(move |v| {
            let len = length.item;
            let character = character.item.clone();
            if column_paths.is_empty() {
                ReturnSuccess::value(action(&v, len, character, v.tag())?)
            } else {
                let mut ret = v;

                for path in &column_paths {
                    let str_clone = character.clone();
                    ret = ret.swap_data_by_column_path(
                        path,
                        Box::new(move |old| action(old, len, str_clone, old.tag())),
                    )?;
                }

                ReturnSuccess::value(ret)
            }
        })
        .to_output_stream())
}

fn action(
    input: &Value,
    length: usize,
    character: String,
    tag: impl Into<Tag>,
) -> Result<Value, ShellError> {
    match &input.value {
        UntaggedValue::Primitive(Primitive::String(s)) => {
            if length < s.len() {
                Ok(
                    UntaggedValue::string(s.chars().take(length).collect::<String>())
                        .into_value(tag),
                )
            } else {
                let mut res = character.repeat(length - s.chars().count());
                res += s.as_ref();
                Ok(UntaggedValue::string(res).into_value(tag))
            }
        }
        other => {
            let got = format!("got {}", other.type_name());
            Err(ShellError::labeled_error(
                "value is not string",
                got,
                tag.into().span,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{action, SubCommand};
    use nu_errors::ShellError;
    use nu_protocol::UntaggedValue;
    use nu_source::Tag;
    use nu_test_support::value::string;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        Ok(test_examples(SubCommand {})?)
    }

    #[test]
    fn left_pad_with_zeros() {
        let word = string("123");
        let pad_char = '0'.to_string();
        let pad_len = 10;
        let expected = UntaggedValue::string("0000000123").into_untagged_value();

        let actual = action(&word, pad_len, pad_char, Tag::unknown()).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn left_pad_but_truncate() {
        let word = string("123456789");
        let pad_char = '0'.to_string();
        let pad_len = 3;
        let expected = UntaggedValue::string("123").into_untagged_value();

        let actual = action(&word, pad_len, pad_char, Tag::unknown()).unwrap();
        assert_eq!(actual, expected);
    }
}
