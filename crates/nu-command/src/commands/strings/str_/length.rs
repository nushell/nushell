use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::ShellTypeName;
use nu_protocol::{
    ColumnPath, Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};

pub struct SubCommand;

struct Arguments {
    column_paths: Vec<ColumnPath>,
}

impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "str length"
    }

    fn signature(&self) -> Signature {
        Signature::build("str length").rest(
            "rest",
            SyntaxShape::ColumnPath,
            "optionally find length of text by column paths",
        )
    }

    fn usage(&self) -> &str {
        "outputs the lengths of the strings in the pipeline"
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        operate(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Return the lengths of multiple strings",
                example: "echo 'hello' | str length",
                result: Some(vec![UntaggedValue::int(5).into_untagged_value()]),
            },
            Example {
                description: "Return the lengths of multiple strings",
                example: "echo 'hi' 'there' | str length",
                result: Some(vec![
                    UntaggedValue::int(2).into_untagged_value(),
                    UntaggedValue::int(5).into_untagged_value(),
                ]),
            },
        ]
    }
}

fn operate(args: CommandArgs) -> Result<ActionStream, ShellError> {
    let (options, input) = (
        Arguments {
            column_paths: args.rest(0)?,
        },
        args.input,
    );

    Ok(input
        .map(move |v| {
            if options.column_paths.is_empty() {
                ReturnSuccess::value(action(&v, v.tag())?)
            } else {
                let mut ret = v;

                for path in &options.column_paths {
                    ret = ret.swap_data_by_column_path(
                        path,
                        Box::new(move |old| action(old, old.tag())),
                    )?;
                }

                ReturnSuccess::value(ret)
            }
        })
        .into_action_stream())
}

fn action(input: &Value, tag: impl Into<Tag>) -> Result<Value, ShellError> {
    match &input.value {
        UntaggedValue::Primitive(Primitive::String(s)) => {
            Ok(UntaggedValue::int(s.len() as i64).into_value(tag))
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
    use super::SubCommand;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }
}
