use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::ShellTypeName;
use nu_protocol::{
    ColumnPath, Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};
use nu_source::{Tag, Tagged};
use nu_value_ext::ValueExt;

pub struct SubCommand;

struct Arguments {
    pattern: Tagged<String>,
    column_paths: Vec<ColumnPath>,
}

impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "str ends-with"
    }

    fn signature(&self) -> Signature {
        Signature::build("str ends-with")
            .required("pattern", SyntaxShape::String, "the pattern to match")
            .rest(
                "rest",
                SyntaxShape::ColumnPath,
                "optionally matches suffix of text by column paths",
            )
    }

    fn usage(&self) -> &str {
        "checks if string ends with pattern"
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        operate(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Checks if string ends with '.rb' pattern",
            example: "echo 'my_library.rb' | str ends-with '.rb'",
            result: Some(vec![UntaggedValue::boolean(true).into_untagged_value()]),
        }]
    }
}

fn operate(args: CommandArgs) -> Result<ActionStream, ShellError> {
    let (options, input) = (
        Arc::new(Arguments {
            pattern: args.req(0)?,
            column_paths: args.rest(1)?,
        }),
        args.input,
    );

    Ok(input
        .map(move |v| {
            if options.column_paths.is_empty() {
                ReturnSuccess::value(action(&v, &options.pattern, v.tag())?)
            } else {
                let mut ret = v;

                for path in &options.column_paths {
                    let options = options.clone();

                    ret = ret.swap_data_by_column_path(
                        path,
                        Box::new(move |old| action(old, &options.pattern, old.tag())),
                    )?;
                }

                ReturnSuccess::value(ret)
            }
        })
        .into_action_stream())
}

fn action(input: &Value, pattern: &str, tag: impl Into<Tag>) -> Result<Value, ShellError> {
    match &input.value {
        UntaggedValue::Primitive(Primitive::String(s)) => {
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
    fn str_ends_with_pattern() {
        let word = string("Cargo.toml");
        let pattern = ".toml";
        let expected = UntaggedValue::boolean(true).into_untagged_value();

        let actual = action(&word, pattern, Tag::unknown()).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn str_does_not_end_with_pattern() {
        let word = string("Cargo.toml");
        let pattern = "Car";
        let expected = UntaggedValue::boolean(false).into_untagged_value();

        let actual = action(&word, pattern, Tag::unknown()).unwrap();
        assert_eq!(actual, expected);
    }
}
