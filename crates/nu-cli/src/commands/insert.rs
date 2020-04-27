use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_value_ext::ValueExt;

pub struct Insert;

#[derive(Deserialize)]
pub struct InsertArgs {
    column: ColumnPath,
    value: Value,
}

impl WholeStreamCommand for Insert {
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
        "Insert a new column with a given value."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, insert)?.run()
    }
}

fn insert(
    InsertArgs { column, value }: InsertArgs,
    RunnableContext { input, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let mut input = input;

    let stream = async_stream! {
        match input.next().await {
            Some(obj @ Value {
                value: UntaggedValue::Row(_),
                ..
            }) => match obj.insert_data_at_column_path(&column, value.clone()) {
                Ok(v) => yield Ok(ReturnSuccess::Value(v)),
                Err(err) => yield Err(err),
            },

            Some(Value { tag, ..}) => {
                yield Err(ShellError::labeled_error(
                    "Unrecognized type in stream",
                    "original value",
                    tag,
                ));
            }

            None => {}
        };

    };
    Ok(stream.to_output_stream())
}
