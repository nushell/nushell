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
        insert(args, registry)
    }
}

fn insert(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();

    let stream = async_stream! {
        let (InsertArgs { column, value }, mut input) = args.process(&registry).await?;
        while let Some(row) = input.next().await {
            match row {
                Value {
                    value: UntaggedValue::Row(_),
                    ..
                } => match row.insert_data_at_column_path(&column, value.clone()) {
                    Ok(v) => yield Ok(ReturnSuccess::Value(v)),
                    Err(err) => yield Err(err),
                },

                Value { tag, ..} => {
                    yield Err(ShellError::labeled_error(
                        "Unrecognized type in stream",
                        "original value",
                        tag,
                    ));
                }

            }
        };

    };
    Ok(stream.to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::Insert;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Insert {})
    }
}
