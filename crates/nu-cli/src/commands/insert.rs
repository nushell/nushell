use crate::commands::classified::block::run_block;
use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use futures::stream::once;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, ReturnSuccess, Scope, Signature, SyntaxShape, UntaggedValue, Value};
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
            .required("value", SyntaxShape::Any, "the value to give the cell(s)")
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

async fn process_row(
    scope: Arc<Scope>,
    mut context: Arc<Context>,
    input: Value,
    mut value: Arc<Value>,
    column: Arc<ColumnPath>,
) -> Result<OutputStream, ShellError> {
    let value = Arc::make_mut(&mut value);

    Ok(match value {
        Value {
            value: UntaggedValue::Block(block),
            ..
        } => {
            let for_block = input.clone();
            let input_stream = once(async { Ok(for_block) }).to_input_stream();

            let result = run_block(
                &block,
                Arc::make_mut(&mut context),
                input_stream,
                &input,
                &scope.vars,
                &scope.env,
            )
            .await;

            match result {
                Ok(mut stream) => {
                    let errors = context.get_errors();
                    if let Some(error) = errors.first() {
                        return Err(error.clone());
                    }

                    match input {
                        obj
                        @
                        Value {
                            value: UntaggedValue::Row(_),
                            ..
                        } => {
                            if let Some(result) = stream.next().await {
                                match obj.insert_data_at_column_path(&column, result) {
                                    Ok(v) => OutputStream::one(ReturnSuccess::value(v)),
                                    Err(e) => OutputStream::one(Err(e)),
                                }
                            } else {
                                OutputStream::empty()
                            }
                        }
                        Value { tag, .. } => OutputStream::one(Err(ShellError::labeled_error(
                            "Unrecognized type in stream",
                            "original value",
                            tag,
                        ))),
                    }
                }
                Err(e) => OutputStream::one(Err(e)),
            }
        }
        _ => match input {
            obj
            @
            Value {
                value: UntaggedValue::Row(_),
                ..
            } => match obj.insert_data_at_column_path(&column, value.clone()) {
                Ok(v) => OutputStream::one(ReturnSuccess::value(v)),
                Err(e) => OutputStream::one(Err(e)),
            },
            Value { tag, .. } => OutputStream::one(Err(ShellError::labeled_error(
                "Unrecognized type in stream",
                "original value",
                tag,
            ))),
        },
    })
}

async fn insert(
    raw_args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let scope = Arc::new(raw_args.call_info.scope.clone());
    let context = Arc::new(Context::from_raw(&raw_args, &registry));
    let (InsertArgs { column, value }, input) = raw_args.process(&registry).await?;
    let value = Arc::new(value);
    let column = Arc::new(column);

    Ok(input
        .then(move |input| {
            let scope = scope.clone();
            let context = context.clone();
            let value = value.clone();
            let column = column.clone();

            async {
                match process_row(scope, context, input, value, column).await {
                    Ok(s) => s,
                    Err(e) => OutputStream::one(Err(e)),
                }
            }
        })
        .flatten()
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
