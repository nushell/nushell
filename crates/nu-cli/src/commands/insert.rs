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

#[async_trait]
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

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        insert(args, registry).await
    }
}

async fn insert(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();

    let (InsertArgs { column, value }, input) = args.process(&registry).await?;

    Ok(input
        .map(move |row| match row {
            Value {
                value: UntaggedValue::Row(_),
                ..
            } => match row.insert_data_at_column_path(&column, value.clone()) {
                Ok(v) => Ok(ReturnSuccess::Value(v)),
                Err(err) => Err(err),
            },

            Value { tag, .. } => Err(ShellError::labeled_error(
                "Unrecognized type in stream",
                "original value",
                tag,
            )),
        })
        .to_output_stream())
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
