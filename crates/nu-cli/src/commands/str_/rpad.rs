use crate::commands::WholeStreamCommand;
use crate::prelude::*;
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
    character: Tagged<char>,
    rest: Vec<ColumnPath>,
}

pub struct SubCommand;

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "str rpad"
    }

    fn signature(&self) -> Signature {
        Signature::build("str rpad")
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

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        operate(args, registry).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Right pad a string with a character a number of places",
                example: "echo 'nushell' | str rpad -l 10 -c '*'",
                result: Some(vec![
                    UntaggedValue::string("nushell***").into_untagged_value()
                ]),
            },
            Example {
                description: "Right pad a string with a character a number of places",
                example: "echo '123' | str rpad -l 10 -c '0'",
                result: Some(vec![
                    UntaggedValue::string("1230000000").into_untagged_value()
                ]),
            },
            Example {
                description: "Use rpad to truncate a string",
                example: "echo '123456789' | str rpad -l 3 -c '0'",
                result: Some(vec![UntaggedValue::string("123").into_untagged_value()]),
            },
        ]
    }
}

async fn operate(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();

    let (
        Arguments {
            length,
            character,
            rest,
        },
        input,
    ) = args.process(&registry).await?;
    let column_paths: Vec<_> = rest;

    Ok(input
        .map(move |v| {
            let len = length.item;
            let character = character.item;
            if column_paths.is_empty() {
                ReturnSuccess::value(action(&v, len, character, v.tag())?)
            } else {
                let mut ret = v;

                for path in &column_paths {
                    ret = ret.swap_data_by_column_path(
                        path,
                        Box::new(move |old| action(old, len, character, old.tag())),
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
    character: char,
    tag: impl Into<Tag>,
) -> Result<Value, ShellError> {
    match &input.value {
        UntaggedValue::Primitive(Primitive::Line(s))
        | UntaggedValue::Primitive(Primitive::String(s)) => {
            if length < s.len() {
                Ok(UntaggedValue::string(&s[0..length]).into_value(tag))
            } else {
                let mut res = s.to_string();
                res += character.to_string().repeat(length - s.len()).as_str();
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
    use nu_plugin::test_helpers::value::string;
    use nu_protocol::UntaggedValue;
    use nu_source::Tag;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        Ok(test_examples(SubCommand {})?)
    }

    #[test]
    fn right_pad_with_zeros() {
        let word = string("123");
        let pad_char = '0';
        let pad_len = 10;
        let expected = UntaggedValue::string("1230000000").into_untagged_value();

        let actual = action(&word, pad_len, pad_char, Tag::unknown()).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn right_pad_but_truncate() {
        let word = string("123456789");
        let pad_char = '0';
        let pad_len = 3;
        let expected = UntaggedValue::string("123").into_untagged_value();

        let actual = action(&word, pad_len, pad_char, Tag::unknown()).unwrap();
        assert_eq!(actual, expected);
    }
}
