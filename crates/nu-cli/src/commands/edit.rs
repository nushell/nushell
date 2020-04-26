use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_value_ext::ValueExt;

pub struct Edit;

#[derive(Deserialize)]
pub struct EditArgs {
    field: ColumnPath,
    replacement: Value,
}

impl WholeStreamCommand for Edit {
    fn name(&self) -> &str {
        "edit"
    }

    fn signature(&self) -> Signature {
        Signature::build("edit")
            .required(
                "field",
                SyntaxShape::ColumnPath,
                "the name of the column to edit",
            )
            .required(
                "replacement value",
                SyntaxShape::Any,
                "the new value to give the cell(s)",
            )
    }

    fn usage(&self) -> &str {
        "Edit an existing column to have a new value."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, edit)?.run()
    }
}

fn edit(
    EditArgs { field, replacement }: EditArgs,
    RunnableContext { input, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let mut input = input;

    let stream = async_stream! {
        match input.next().await {
            Some(obj @ Value {
                value: UntaggedValue::Row(_),
                ..
            }) => match obj.replace_data_at_column_path(&field, replacement.clone()) {
                Some(v) => yield Ok(ReturnSuccess::Value(v)),
                None => {
                    yield Err(ShellError::labeled_error(
                        "edit could not find place to insert column",
                        "column name",
                        obj.tag,
                    ))
                }
            },

            Some(Value { tag, ..}) => {
                yield Err(ShellError::labeled_error(
                    "Unrecognized type in stream",
                    "original value",
                    tag,
                ))
            }
            _ => {}
        }
    };

    Ok(stream.to_output_stream())
}
