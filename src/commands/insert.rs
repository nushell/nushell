use crate::commands::PerItemCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{CallInfo, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};

pub struct Insert;

impl PerItemCommand for Insert {
    fn name(&self) -> &str {
        "insert"
    }

    fn signature(&self) -> Signature {
        Signature::build("insert")
            .required(
                "column",
                SyntaxShape::ColumnPath,
                "the column name to insert",
            )
            .required(
                "value",
                SyntaxShape::String,
                "the value to give the cell(s)",
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
        let field = call_info.args.expect_nth(0)?.as_column_path().unwrap();
        let replacement = call_info.args.expect_nth(1)?.tagged_unknown();

        let stream = match value {
            obj @ Value {
                value: UntaggedValue::Row(_),
                ..
            } => match obj.insert_data_at_column_path(&field, replacement.item.clone()) {
                Ok(v) => VecDeque::from(vec![Ok(ReturnSuccess::Value(v))]),
                Err(err) => return Err(err),
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

/*
use nu::{serve_plugin, Plugin, ValueExt};
use nu_errors::ShellError;
use nu_protocol::{
    CallInfo, ColumnPath, Primitive, ReturnSuccess, ReturnValue, Signature, SpannedTypeName,
    SyntaxShape, UntaggedValue, Value,
};
use nu_source::Tagged;

struct Edit {
    field: Option<Tagged<ColumnPath>>,
    value: Option<UntaggedValue>,
}
impl Edit {
    fn new() -> Edit {
        Edit {
            field: None,
            value: None,
        }
    }

    fn edit(&self, value: Value) -> Result<Value, ShellError> {
        let value_tag = value.tag();
        match (value, self.value.clone()) {
            (
                obj @ Value {
                    value: UntaggedValue::Row(_),
                    ..
                },
                Some(v),
            ) => match &self.field {
                Some(f) => {
                    match obj.replace_data_at_column_path(&f, v.clone().into_untagged_value()) {
                        Some(v) => return Ok(v),
                        None => {
                            return Err(ShellError::labeled_error(
                                "edit could not find place to insert column",
                                "column name",
                                &f.tag,
                            ))
                        }
                    }
                }
                None => Err(ShellError::untagged_runtime_error(
                    "edit needs a column when changing a value in a table",
                )),
            },
            _ => Err(ShellError::labeled_error(
                "Unrecognized type in stream",
                "original value",
                value_tag,
            )),
        }
    }
}

impl Plugin for Edit {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("edit")
            .desc("Edit an existing column to have a new value.")
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
            .filter())
    }

    fn begin_filter(&mut self, call_info: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        if let Some(args) = call_info.args.positional {
            match &args[0] {
                table @ Value {
                    value: UntaggedValue::Primitive(Primitive::ColumnPath(_)),
                    ..
                } => {
                    self.field = Some(table.as_column_path()?);
                }
                value => return Err(ShellError::type_error("table", value.spanned_type_name())),
            }

            match &args[1] {
                Value { value: v, .. } => {
                    self.value = Some(v.clone());
                }
            }
        }

        Ok(vec![])
    }

    fn filter(&mut self, input: Value) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(vec![ReturnSuccess::value(self.edit(input)?)])
    }
}

fn main() {
    serve_plugin(&mut Edit::new());
}
*/
