use crate::commands::PerItemCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{CallInfo, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_value_ext::ValueExt;

pub struct Edit;

impl PerItemCommand for Edit {
    fn name(&self) -> &str {
        "edit"
    }

    fn signature(&self) -> Signature {
        Signature::build("edit")
            .required(
                "Field",
                SyntaxShape::ColumnPath,
                "the name of the column to edit",
            )
            .required(
                "Value",
                SyntaxShape::String,
                "the new value to give the cell(s)",
            )
    }

    fn usage(&self) -> &str {
        "Edit an existing column to have a new value."
    }

    fn run(
        &self,
        call_info: &CallInfo,
        _registry: &CommandRegistry,
        _raw_args: &RawCommandArgs,
        value: Value,
    ) -> Result<OutputStream, ShellError> {
        let value_tag = value.tag();
        let field = call_info.args.expect_nth(0)?.as_column_path()?;
        let replacement = call_info.args.expect_nth(1)?.tagged_unknown();

        let stream = match value {
            obj @ Value {
                value: UntaggedValue::Row(_),
                ..
            } => match obj.replace_data_at_column_path(&field, replacement.item.clone()) {
                Some(v) => VecDeque::from(vec![Ok(ReturnSuccess::Value(v))]),
                None => {
                    return Err(ShellError::labeled_error(
                        "edit could not find place to insert column",
                        "column name",
                        &field.tag,
                    ))
                }
            },

            _ => {
                return Err(ShellError::labeled_error(
                    "Unrecognized type in stream",
                    "original value",
                    value_tag,
                ))
            }
        };

        Ok(stream.to_output_stream())
    }
}
