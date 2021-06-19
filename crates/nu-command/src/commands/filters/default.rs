use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;
use nu_value_ext::ValueExt;

pub struct Default;

impl WholeStreamCommand for Default {
    fn name(&self) -> &str {
        "default"
    }

    fn signature(&self) -> Signature {
        Signature::build("default")
            .required("column name", SyntaxShape::String, "the name of the column")
            .required(
                "column value",
                SyntaxShape::Any,
                "the value of the column to default",
            )
    }

    fn usage(&self) -> &str {
        "Sets a default row's column if missing."
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        default(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Give a default 'target' to all file entries",
            example: "ls -la | default target 'nothing'",
            result: None,
        }]
    }
}

fn default(args: CommandArgs) -> Result<ActionStream, ShellError> {
    let column: Tagged<String> = args.req(0)?;
    let value: Value = args.req(1)?;

    let input = args.input;

    Ok(input
        .map(move |item| {
            let should_add = matches!(
                item,
                Value {
                    value: UntaggedValue::Row(ref r),
                    ..
                } if r.get_data(&column.item).borrow().is_none()
            );

            if should_add {
                match item.insert_data_at_path(&column.item, value.clone()) {
                    Some(new_value) => ReturnSuccess::value(new_value),
                    None => ReturnSuccess::value(item),
                }
            } else {
                ReturnSuccess::value(item)
            }
        })
        .into_action_stream())
}

#[cfg(test)]
mod tests {
    use super::Default;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Default {})
    }
}
