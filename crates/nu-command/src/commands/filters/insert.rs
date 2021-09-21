use crate::prelude::*;
use nu_engine::run_block;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::hir::ExternalRedirection;
use nu_protocol::{
    ColumnPath, Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};
use nu_value_ext::ValueExt;

pub struct Command;

impl WholeStreamCommand for Command {
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

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        insert(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Insert a column with a value",
            example: "echo [[author, commits]; ['Andrés', 1]] | insert branches 5",
            result: Some(vec![UntaggedValue::row(indexmap! {
                    "author".to_string() => Value::from("Andrés"),
                    "commits".to_string() => UntaggedValue::int(1).into(),
                    "branches".to_string() => UntaggedValue::int(5).into(),
            })
            .into()]),
        },Example {
            description: "Use in block form for more involved insertion logic",
            example: "echo [[author, lucky_number]; ['Yehuda', 4]] | insert success { $it.lucky_number * 10 }",
            result: Some(vec![UntaggedValue::row(indexmap! {
                    "author".to_string() => Value::from("Yehuda"),
                    "lucky_number".to_string() => UntaggedValue::int(4).into(),
                    "success".to_string() => UntaggedValue::int(40).into(),
            })
            .into()]),
        }]
    }
}

fn process_row(
    context: Arc<EvaluationContext>,
    input: Value,
    mut value: Arc<Value>,
    field: Arc<ColumnPath>,
) -> Result<ActionStream, ShellError> {
    let value = Arc::make_mut(&mut value);

    Ok(match value {
        Value {
            value: UntaggedValue::Block(block),
            tag: block_tag,
        } => {
            let for_block = input.clone();
            let input_stream = vec![Ok(for_block)].into_iter().into_input_stream();

            context.scope.enter_scope();
            context.scope.add_vars(&block.captured.entries);
            if let Some((arg, _)) = block.block.params.positional.first() {
                context.scope.add_var(arg.name(), input.clone());
            }

            let result = run_block(
                &block.block,
                &context,
                input_stream,
                ExternalRedirection::Stdout,
            );

            context.scope.exit_scope();

            match result {
                Ok(mut stream) => {
                    let values = stream.drain_vec();

                    let errors = context.get_errors();
                    if let Some(error) = errors.first() {
                        return Err(error.clone());
                    }

                    let result = if values.len() == 1 {
                        let value = values
                            .get(0)
                            .ok_or_else(|| ShellError::unexpected("No value to insert with."))?;

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
                        } => match obj.insert_data_at_column_path(&field, result) {
                            Ok(v) => ActionStream::one(ReturnSuccess::value(v)),
                            Err(e) => ActionStream::one(Err(e)),
                        },
                        _ => ActionStream::one(Err(ShellError::labeled_error(
                            "Unrecognized type in stream",
                            "original value",
                            block_tag.clone(),
                        ))),
                    }
                }
                Err(e) => ActionStream::one(Err(e)),
            }
        }
        value => match input {
            Value {
                value: UntaggedValue::Primitive(Primitive::Nothing),
                ..
            } => match context
                .scope
                .get_var("$it")
                .unwrap_or_else(|| UntaggedValue::nothing().into_untagged_value())
                .insert_data_at_column_path(&field, value.clone())
            {
                Ok(v) => ActionStream::one(ReturnSuccess::value(v)),
                Err(e) => ActionStream::one(Err(e)),
            },
            _ => match input.insert_data_at_column_path(&field, value.clone()) {
                Ok(v) => ActionStream::one(ReturnSuccess::value(v)),
                Err(e) => ActionStream::one(Err(e)),
            },
        },
    })
}

fn insert(args: CommandArgs) -> Result<ActionStream, ShellError> {
    let context = Arc::new(args.context.clone());
    let column: ColumnPath = args.req(0)?;
    let value: Value = args.req(1)?;
    let input = args.input;

    let value = Arc::new(value);
    let column = Arc::new(column);

    Ok(input
        .flat_map(move |input| {
            let context = context.clone();
            let value = value.clone();
            let column = column.clone();

            match process_row(context, input, value, column) {
                Ok(s) => s,
                Err(e) => ActionStream::one(Err(e)),
            }
        })
        .into_action_stream())
}
