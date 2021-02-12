use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::ShellTypeName;
use nu_protocol::{
    ColumnPath, Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};
use nu_source::Tag;
use nu_value_ext::ValueExt;

#[derive(Deserialize)]
struct Arguments {
    rest: Vec<ColumnPath>,
}

pub struct SubCommand;

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "str capitalize"
    }

    fn signature(&self) -> Signature {
        Signature::build("str capitalize").rest(
            SyntaxShape::ColumnPath,
            "optionally capitalize text by column paths",
        )
    }

    fn usage(&self) -> &str {
        "capitalizes text"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        operate(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Capitalize contents",
            example: "echo 'good day' | str capitalize",
            result: Some(vec![Value::from("Good day")]),
        }]
    }
}

async fn operate(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let (Arguments { rest }, input) = args.process().await?;

    let column_paths: Vec<_> = rest;

    Ok(input
        .map(move |v| {
            if column_paths.is_empty() {
                ReturnSuccess::value(action(&v, v.tag())?)
            } else {
                let mut ret = v;

                for path in &column_paths {
                    ret = ret.swap_data_by_column_path(
                        path,
                        Box::new(move |old| action(old, old.tag())),
                    )?;
                }

                ReturnSuccess::value(ret)
            }
        })
        .to_output_stream())
}

fn action(input: &Value, tag: impl Into<Tag>) -> Result<Value, ShellError> {
    match &input.value {
        UntaggedValue::Primitive(Primitive::String(s)) => {
            let mut capitalized = String::new();

            for (idx, character) in s.chars().enumerate() {
                let out = if idx == 0 {
                    character.to_uppercase().to_string()
                } else {
                    character.to_lowercase().to_string()
                };

                capitalized.push_str(&out);
            }

            Ok(UntaggedValue::string(capitalized).into_value(tag))
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
    use super::ShellError;
    use super::{action, SubCommand};
    use nu_source::Tag;
    use nu_test_support::value::string;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }

    #[test]
    fn capitalizes() {
        let word = string("andres");
        let expected = string("Andres");

        let actual = action(&word, Tag::unknown()).unwrap();
        assert_eq!(actual, expected);
    }
}
