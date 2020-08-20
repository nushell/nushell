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
    pattern: Tagged<String>,
    rest: Vec<ColumnPath>,
}
pub struct SubCommand;

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "str ends-with"
    }

    fn signature(&self) -> Signature {
        Signature::build("str ends-with")
            .required("pattern", SyntaxShape::String, "the pattern to match")
            .rest(
                SyntaxShape::ColumnPath,
                "optionally matches suffix of text by column paths",
            )
    }

    fn usage(&self) -> &str {
        "checks if string ends with pattern"
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
            description: "Checks if string ends with '.rb' pattern",
            example: "echo 'my_library.rb' | str ends-with '.rb'",
            result: Some(vec![UntaggedValue::boolean(true).into_untagged_value()]),
        }]
    }
}

async fn operate(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();

    let (Arguments { pattern, rest }, input) = args.process(&registry).await?;

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
        UntaggedValue::Primitive(Primitive::Line(s))
        | UntaggedValue::Primitive(Primitive::String(s)) => {
            let ends_with = s.ends_with(pattern);
            Ok(UntaggedValue::boolean(ends_with).into_value(tag))
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
    use nu_protocol::{Primitive, UntaggedValue};
    use nu_source::Tag;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }

    #[test]
    fn str_ends_with_pattern() {
        let word = string("Cargo.toml");
        let pattern = ".toml";
        let expected =
            UntaggedValue::Primitive(Primitive::Boolean(true.into())).into_untagged_value();

        let actual = action(&word, &pattern, Tag::unknown()).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn str_does_not_end_with_pattern() {
        let word = string("Cargo.toml");
        let pattern = "Car";
        let expected =
            UntaggedValue::Primitive(Primitive::Boolean(false.into())).into_untagged_value();

        let actual = action(&word, &pattern, Tag::unknown()).unwrap();
        assert_eq!(actual, expected);
    }
}
