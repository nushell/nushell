use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, UntaggedValue, Value};

pub struct Command;

impl WholeStreamCommand for Command {
    fn name(&self) -> &str {
        "append"
    }

    fn signature(&self) -> Signature {
        Signature::build("append").required(
            "row value",
            SyntaxShape::Any,
            "the value of the row to append to the table",
        )
    }

    fn usage(&self) -> &str {
        "Append a row to the table."
    }

    fn run(&self, mut args: CommandArgs) -> Result<OutputStream, ShellError> {
        let mut value: Value = args.req(0)?;

        let mut prepend = vec![];

        if let Some(first) = args.input.next() {
            value.tag = first.tag();
            prepend.push(first);
        }

        // Checks if we are trying to append a row literal
        if let Value {
            value: UntaggedValue::Table(values),
            tag,
        } = &value
        {
            if values.len() == 1 && values[0].is_row() {
                value = values[0].value.clone().into_value(tag);
            }
        }

        Ok(prepend
            .into_iter()
            .chain(args.input.into_iter().chain([value]))
            .into_output_stream())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Add values to the end of the table",
                example: "echo [1 2 3] | append 4",
                result: Some(vec![
                    UntaggedValue::int(1).into(),
                    UntaggedValue::int(2).into(),
                    UntaggedValue::int(3).into(),
                    UntaggedValue::int(4).into(),
                ]),
            },
            Example {
                description: "Add row value to the end of the table",
                example: "echo [[country]; [Ecuador] ['New Zealand']] | append [[country]; [USA]]",
                result: Some(vec![
                    row! { "country".into() => Value::from("Ecuador")},
                    row! { "country".into() => Value::from("New Zealand")},
                    row! { "country".into() => Value::from("USA")},
                ]),
            },
        ]
    }
}
