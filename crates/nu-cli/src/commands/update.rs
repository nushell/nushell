use crate::commands::classified::block::run_block;
use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_value_ext::ValueExt;

use futures::stream::once;
pub struct Update;

#[derive(Deserialize)]
pub struct UpdateArgs {
    field: ColumnPath,
    replacement: Value,
}

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

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        update(args, registry)
    }
}

fn update(raw_args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let scope = raw_args.call_info.scope.clone();

    let stream = async_stream! {
        let mut context = Context::from_raw(&raw_args, &registry);
        let (UpdateArgs { field, replacement }, mut input) = raw_args.process(&registry).await?;
        while let Some(input) = input.next().await {
            let replacement = replacement.clone();
            match replacement {
                Value {
                    value: UntaggedValue::Block(block),
                    tag,
                } =>  {
                    let for_block = input.clone();
                    let input_clone = input.clone();
                    let input_stream = once(async { Ok(for_block) }).to_input_stream();

                    let result = run_block(
                        &block,
                        &mut context,
                        input_stream,
                        &scope.clone().set_it(input_clone),
                    ).await;

                    match result {
                        Ok(mut stream) => {
                            let errors = context.get_errors();
                            if let Some(error) = errors.first() {
                                yield Err(error.clone());
                            }

                            match input {
                                obj @ Value {
                                    value: UntaggedValue::Row(_),
                                    ..
                                } => {
                                    if let Some(result) = stream.next().await {
                                        match obj.replace_data_at_column_path(&field, result.clone()) {
                                            Some(v) => yield Ok(ReturnSuccess::Value(v)),
                                            None => {
                                                yield Err(ShellError::labeled_error(
                                                    "update could not find place to insert column",
                                                    "column name",
                                                    obj.tag,
                                                ))
                                            }
                                        }
                                    }
                                }
                                Value { tag, ..} => {
                                    yield Err(ShellError::labeled_error(
                                        "Unrecognized type in stream",
                                        "original value",
                                        tag,
                                    ))
                                }
                            }
                        }
                        Err(e) => {
                            yield Err(e);
                        }
                    }
                }
                _ => {
                    match input {
                        obj @ Value {
                            value: UntaggedValue::Row(_),
                            ..
                        } => match obj.replace_data_at_column_path(&field, replacement.clone()) {
                            Some(v) => yield Ok(ReturnSuccess::Value(v)),
                            None => {
                                yield Err(ShellError::labeled_error(
                                    "update could not find place to insert column",
                                    "column name",
                                    obj.tag,
                                ))
                            }
                        },
                        Value { tag, ..} => {
                            yield Err(ShellError::labeled_error(
                                "Unrecognized type in stream",
                                "original value",
                                tag,
                            ))
                        }
                        _ => {}
                    }
                }
            }
        }
    };

    Ok(stream.to_output_stream())
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
