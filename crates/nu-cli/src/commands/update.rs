use crate::commands::classified::block::run_block;
use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, ReturnSuccess, Scope, Signature, SyntaxShape, UntaggedValue, Value};
use nu_value_ext::ValueExt;

use futures::stream::once;
pub struct Update;

#[derive(Deserialize)]
pub struct UpdateArgs {
    field: ColumnPath,
    replacement: Value,
}

#[async_trait]
impl WholeStreamCommand for Update {
    fn name(&self) -> &str {
        "update"
    }

    fn signature(&self) -> Signature {
        Signature::build("update")
            .required(
                "field",
                SyntaxShape::ColumnPath,
                "the name of the column to update",
            )
            .required(
                "replacement value",
                SyntaxShape::Any,
                "the new value to give the cell(s)",
            )
    }

    fn usage(&self) -> &str {
        "Update an existing column to have a new value."
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        update(args, registry).await
    }
}

async fn process_row(
    scope: Arc<Scope>,
    mut context: Arc<Context>,
    input: Value,
    mut replacement: Arc<Value>,
    field: Arc<ColumnPath>,
) -> Result<OutputStream, ShellError> {
    let replacement = Arc::make_mut(&mut replacement);

    Ok(match replacement {
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
                                match obj.replace_data_at_column_path(&field, result) {
                                    Some(v) => OutputStream::one(ReturnSuccess::value(v)),
                                    None => OutputStream::one(Err(ShellError::labeled_error(
                                        "update could not find place to insert column",
                                        "column name",
                                        obj.tag,
                                    ))),
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
            } => match obj.replace_data_at_column_path(&field, replacement.clone()) {
                Some(v) => OutputStream::one(ReturnSuccess::value(v)),
                None => OutputStream::one(Err(ShellError::labeled_error(
                    "update could not find place to insert column",
                    "column name",
                    obj.tag,
                ))),
            },
            Value { tag, .. } => OutputStream::one(Err(ShellError::labeled_error(
                "Unrecognized type in stream",
                "original value",
                tag,
            ))),
        },
    })
}

async fn update(
    raw_args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let scope = Arc::new(raw_args.call_info.scope.clone());
    let context = Arc::new(Context::from_raw(&raw_args, &registry));
    let (UpdateArgs { field, replacement }, input) = raw_args.process(&registry).await?;
    let replacement = Arc::new(replacement);
    let field = Arc::new(field);

    Ok(input
        .then(move |input| {
            let scope = scope.clone();
            let context = context.clone();
            let replacement = replacement.clone();
            let field = field.clone();

            async {
                match process_row(scope, context, input, replacement, field).await {
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
    use super::Update;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Update {})
    }
}
