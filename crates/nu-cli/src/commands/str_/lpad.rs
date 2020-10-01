use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::ShellTypeName;
use nu_protocol::{
    ColumnPath, Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};
use nu_source::Tag;
use nu_value_ext::ValueExt;

#[derive(Deserialize)]
struct Arguments {
    length: usize,
    character: char,
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

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        operate(args, registry).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Left pad a string with a character a number of places",
            example: "echo 'nushell' | str lpad -l 10 -c '*'",
            result: Some(vec![
                UntaggedValue::string("***nushell").into_untagged_value()
            ]),
        }]
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
            if column_paths.is_empty() {
                ReturnSuccess::value(action(&v, length, &character, v.tag())?)
            } else {
                let mut ret = v;

                for path in &column_paths {
                    ret = ret.swap_data_by_column_path(
                        path,
                        Box::new(move |old| action(old, length, &character, old.tag())),
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
    character: &char,
    tag: impl Into<Tag>,
) -> Result<Value, ShellError> {
    match &input.value {
        UntaggedValue::Primitive(Primitive::Line(s))
        | UntaggedValue::Primitive(Primitive::String(s)) => {
            if length < s.len() {
                Ok(UntaggedValue::string(&s[0..length]).into_value(tag))
            } else {
                let mut res = character.to_string().repeat(length - s.len());
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
    use nu_plugin::test_helpers::value::string;
    use nu_protocol::UntaggedValue;
    use nu_source::Tag;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }

    #[test]
    fn string_contains_other_string_case_sensitive() {
        let word = string("Cargo.tomL");
        let pattern = ".tomL";
        let insensitive = false;
        let expected = UntaggedValue::boolean(true).into_untagged_value();

        let actual = action(&word, &pattern, insensitive, Tag::unknown()).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn string_does_not_contain_other_string_case_sensitive() {
        let word = string("Cargo.tomL");
        let pattern = "Lomt.";
        let insensitive = false;
        let expected = UntaggedValue::boolean(false).into_untagged_value();

        let actual = action(&word, &pattern, insensitive, Tag::unknown()).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn string_contains_other_string_case_insensitive() {
        let word = string("Cargo.ToMl");
        let pattern = ".TOML";
        let insensitive = true;
        let expected = UntaggedValue::boolean(true).into_untagged_value();

        let actual = action(&word, &pattern, insensitive, Tag::unknown()).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn string_does_not_contain_other_string_case_insensitive() {
        let word = string("Cargo.tOml");
        let pattern = "lomt.";
        let insensitive = true;
        let expected = UntaggedValue::boolean(false).into_untagged_value();

        let actual = action(&word, &pattern, insensitive, Tag::unknown()).unwrap();
        assert_eq!(actual, expected);
    }
}
