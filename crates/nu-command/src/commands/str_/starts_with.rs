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
    pattern: Tagged<String>,
    rest: Vec<ColumnPath>,
}
pub struct SubCommand;

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "str starts-with"
    }

    fn signature(&self) -> Signature {
        Signature::build("str starts-with")
            .required("pattern", SyntaxShape::String, "the pattern to match")
            .rest(
                SyntaxShape::ColumnPath,
                "optionally matches prefix of text by column paths",
            )
    }

    fn usage(&self) -> &str {
        "checks if string starts with pattern"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        operate(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Checks if string starts with 'my' pattern",
            example: "echo 'my_library.rb' | str starts-with 'my'",
            result: Some(vec![UntaggedValue::boolean(true).into_untagged_value()]),
        }]
    }
}

async fn operate(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let (Arguments { pattern, rest }, input) = args.process().await?;

    let column_paths: Vec<_> = rest;

    Ok(input
        .map(move |v| {
            if column_paths.is_empty() {
                ReturnSuccess::value(action(&v, &pattern, v.tag())?)
            } else {
                let mut ret = v;

                for path in &column_paths {
                    let pattern = pattern.clone();
                    ret = ret.swap_data_by_column_path(
                        path,
                        Box::new(move |old| action(old, &pattern, old.tag())),
                    )?;
                }

                ReturnSuccess::value(ret)
            }
        })
        .to_output_stream())
}

fn action(input: &Value, pattern: &str, tag: impl Into<Tag>) -> Result<Value, ShellError> {
    match &input.value {
        UntaggedValue::Primitive(Primitive::String(s)) => {
            let starts_with = s.starts_with(pattern);
            Ok(UntaggedValue::boolean(starts_with).into_value(tag))
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
    use nu_protocol::UntaggedValue;
    use nu_source::Tag;
    use nu_test_support::value::string;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }

    #[test]
    fn str_starts_with_pattern() {
        let word = string("Cargo.toml");
        let pattern = "Car";
        let expected = UntaggedValue::boolean(true).into_untagged_value();

        let actual = action(&word, &pattern, Tag::unknown()).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn str_does_not_start_with_pattern() {
        let word = string("Cargo.toml");
        let pattern = ".toml";
        let expected = UntaggedValue::boolean(false).into_untagged_value();

        let actual = action(&word, &pattern, Tag::unknown()).unwrap();
        assert_eq!(actual, expected);
    }
}
