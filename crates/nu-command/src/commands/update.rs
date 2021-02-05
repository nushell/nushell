use crate::prelude::*;
use nu_engine::run_block;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    ColumnPath, Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};
use nu_source::HasFallibleSpan;
use nu_value_ext::ValueExt;

use futures::stream::once;

pub struct Command;

#[derive(Deserialize)]
pub struct Arguments {
    field: ColumnPath,
    replacement: Value,
}

#[async_trait]
impl WholeStreamCommand for Command {
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

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        update(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Update a column value",
            example: "echo [[name, stars]; ['nu', 5]] | update name 'Nushell'",
            result: Some(vec![UntaggedValue::row(indexmap! {
                    "name".to_string() => Value::from("Nushell"),
                    "stars".to_string() => UntaggedValue::int(5).into(),
            })
            .into()]),
        },Example {
            description: "Use in block form for more involved updating logic",
            example: "echo [[project, authors]; ['nu', ['Andrés', 'Jonathan', 'Yehuda']]] | update authors { get authors | str collect ',' }",
            result: Some(vec![UntaggedValue::row(indexmap! {
                    "project".to_string() => Value::from("nu"),
                    "authors".to_string() => Value::from("Andrés,Jonathan,Yehuda"),
            })
            .into()]),
        }]
    }
}

async fn process_row(
    context: Arc<EvaluationContext>,
    input: Value,
    mut replacement: Arc<Value>,
    field: Arc<ColumnPath>,
    tag: Arc<Tag>,
) -> Result<OutputStream, ShellError> {
    let tag = &*tag;
    let replacement = Arc::make_mut(&mut replacement);

    Ok(match replacement {
        Value {
            value: UntaggedValue::Block(captured_block),
            tag: block_tag,
        } => {
            let for_block = input.clone();
            let input_stream = once(async { Ok(for_block) }).to_input_stream();

            context.scope.enter_scope();
            context.scope.add_var("$it", input.clone());
            context.scope.add_vars(&captured_block.captured.entries);

            let result = run_block(&captured_block.block, &*context, input_stream).await;

            context.scope.exit_scope();

            match result {
                Ok(mut stream) => {
                    let values = stream.drain_vec().await;

                    let errors = context.get_errors();
                    if let Some(error) = errors.first() {
                        return Err(error.clone());
                    }

                    let result = if values.len() == 1 {
                        let value = values
                            .get(0)
                            .ok_or_else(|| ShellError::unexpected("No value to update with."))?;

                        Value {
                            value: value.value.clone(),
                            tag: input.tag.clone(),
                        }
                    } else if values.is_empty() {
                        UntaggedValue::nothing().into_value(&input.tag)
                    } else {
                        UntaggedValue::table(&values).into_value(&input.tag)
                    };

                    match input {
                        obj
                        @
                        Value {
                            value: UntaggedValue::Row(_),
                            ..
                        } => match obj.replace_data_at_column_path(&field, result) {
                            Some(v) => OutputStream::one(ReturnSuccess::value(v)),
                            None => OutputStream::one(Err(ShellError::labeled_error(
                                "update could not find place to insert column",
                                "column name",
                                obj.tag,
                            ))),
                        },
                        _ => OutputStream::one(Err(ShellError::labeled_error(
                            "Unrecognized type in stream",
                            "original value",
                            block_tag.clone(),
                        ))),
                    }
                }
                Err(e) => OutputStream::one(Err(e)),
            }
        }
        replacement => match input {
            Value {
                value: UntaggedValue::Primitive(Primitive::Nothing),
                ..
            } => match context
                .scope
                .get_var("$it")
                .unwrap_or_else(|| UntaggedValue::nothing().into_untagged_value())
                .replace_data_at_column_path(&field, replacement.clone())
            {
                Some(v) => OutputStream::one(ReturnSuccess::value(v)),
                None => OutputStream::one(Err(ShellError::labeled_error(
                    "update could not find place to insert column",
                    "column name",
                    field.maybe_span().unwrap_or(tag.span),
                ))),
            },
            Value { value: _, ref tag } => {
                match input.replace_data_at_column_path(&field, replacement.clone()) {
                    Some(v) => OutputStream::one(ReturnSuccess::value(v)),
                    None => OutputStream::one(Err(ShellError::labeled_error(
                        "update could not find place to insert column",
                        "column name",
                        field.maybe_span().unwrap_or(tag.span),
                    ))),
                }
            }
        },
    })
}

async fn update(raw_args: CommandArgs) -> Result<OutputStream, ShellError> {
    let name_tag = Arc::new(raw_args.call_info.name_tag.clone());
    let context = Arc::new(EvaluationContext::from_args(&raw_args));
    let (Arguments { field, replacement }, input) = raw_args.process().await?;
    let replacement = Arc::new(replacement);
    let field = Arc::new(field);

    Ok(input
        .then(move |input| {
            let tag = name_tag.clone();
            let context = context.clone();
            let replacement = replacement.clone();
            let field = field.clone();

            async {
                match process_row(context, input, replacement, field, tag).await {
                    Ok(s) => s,
                    Err(e) => OutputStream::one(Err(e)),
                }
            }
        })
        .flatten()
        .to_output_stream())
}
