use crate::prelude::*;
use nu_engine::run_block;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    hir::CapturedBlock, ColumnPath, Primitive, ReturnSuccess, Signature, SyntaxShape,
    UntaggedValue, Value,
};

use crate::utils::arguments::arguments;
use futures::stream::once;
use nu_value_ext::{as_string, ValueExt};

#[derive(Deserialize)]
pub struct Arguments {
    rest: Vec<Value>,
}

pub struct Command;

#[async_trait]
impl WholeStreamCommand for Command {
    fn name(&self) -> &str {
        "empty?"
    }

    fn signature(&self) -> Signature {
        Signature::build("empty?").rest(
            SyntaxShape::Any,
            "the names of the columns to check emptiness. Pass an optional block to replace if empty",
        )
    }

    fn usage(&self) -> &str {
        "Check for empty values"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        is_empty(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Check if a value is empty",
                example: "echo '' | empty?",
                result: Some(vec![UntaggedValue::boolean(true).into()]),
            },
            Example {
                description: "more than one column",
                example: "echo [[meal size]; [arepa small] [taco '']] | empty? meal size",
                result: Some(
                    vec![
                        UntaggedValue::row(indexmap! {
                                "meal".to_string() => Value::from(false),
                                "size".to_string() => Value::from(false),
                        })
                        .into(),
                        UntaggedValue::row(indexmap! {
                                "meal".to_string() => Value::from(false),
                                "size".to_string() => Value::from(true),
                        })
                        .into(),
                    ],
                ),
            },Example {
                description: "use a block if setting the empty cell contents is wanted",
                example: "echo [[2020/04/16 2020/07/10 2020/11/16]; ['' [27] [37]]] | empty? 2020/04/16 { = [33 37] }",
                result: Some(
                    vec![
                        UntaggedValue::row(indexmap! {
                                "2020/04/16".to_string() => UntaggedValue::table(&[UntaggedValue::int(33).into(), UntaggedValue::int(37).into()]).into(),
                                "2020/07/10".to_string() => UntaggedValue::table(&[UntaggedValue::int(27).into()]).into(),
                                "2020/11/16".to_string() => UntaggedValue::table(&[UntaggedValue::int(37).into()]).into(),
                        })
                        .into(),
                    ],
                ),
            },
        ]
    }
}

async fn is_empty(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let name_tag = Arc::new(args.call_info.name_tag.clone());
    let context = Arc::new(EvaluationContext::from_args(&args));
    let (Arguments { mut rest }, input) = args.process().await?;
    let (columns, default_block): (Vec<ColumnPath>, Option<Box<CapturedBlock>>) =
        arguments(&mut rest)?;
    let default_block = Arc::new(default_block);

    if input.is_empty() {
        let stream = futures::stream::iter(vec![
            UntaggedValue::Primitive(Primitive::Nothing).into_value(tag)
        ]);

        return Ok(InputStream::from_stream(stream)
            .then(move |input| {
                let tag = name_tag.clone();
                let context = context.clone();
                let block = default_block.clone();
                let columns = vec![];

                async {
                    match process_row(context, input, block, columns, tag).await {
                        Ok(s) => s,
                        Err(e) => OutputStream::one(Err(e)),
                    }
                }
            })
            .flatten()
            .to_output_stream());
    }

    Ok(input
        .then(move |input| {
            let tag = name_tag.clone();
            let context = context.clone();
            let block = default_block.clone();
            let columns = columns.clone();

            async {
                match process_row(context, input, block, columns, tag).await {
                    Ok(s) => s,
                    Err(e) => OutputStream::one(Err(e)),
                }
            }
        })
        .flatten()
        .to_output_stream())
}

async fn process_row(
    context: Arc<EvaluationContext>,
    input: Value,
    default_block: Arc<Option<Box<CapturedBlock>>>,
    column_paths: Vec<ColumnPath>,
    tag: Arc<Tag>,
) -> Result<OutputStream, ShellError> {
    let _tag = &*tag;
    let mut out = Arc::new(None);
    let results = Arc::make_mut(&mut out);

    if let Some(default_block) = &*default_block {
        let for_block = input.clone();
        let input_stream = once(async { Ok(for_block) }).to_input_stream();

        context.scope.enter_scope();
        context.scope.add_vars(&default_block.captured.entries);
        context.scope.add_var("$it", input.clone());

        let stream = run_block(&default_block.block, &*context, input_stream).await;
        context.scope.exit_scope();

        let mut stream = stream?;
        *results = Some({
            let values = stream.drain_vec().await;

            let errors = context.get_errors();

            if let Some(error) = errors.first() {
                return Err(error.clone());
            }

            if values.len() == 1 {
                let value = values
                    .get(0)
                    .ok_or_else(|| ShellError::unexpected("No value."))?;

                Value {
                    value: value.value.clone(),
                    tag: input.tag.clone(),
                }
            } else if values.is_empty() {
                UntaggedValue::nothing().into_value(&input.tag)
            } else {
                UntaggedValue::table(&values).into_value(&input.tag)
            }
        });
    }

    match input {
        Value {
            value: UntaggedValue::Row(ref r),
            ref tag,
        } => {
            if column_paths.is_empty() {
                Ok(OutputStream::one(ReturnSuccess::value({
                    let is_empty = input.is_empty();

                    if default_block.is_some() {
                        if is_empty {
                            results
                                .clone()
                                .unwrap_or_else(|| UntaggedValue::boolean(true).into_value(tag))
                        } else {
                            input.clone()
                        }
                    } else {
                        UntaggedValue::boolean(is_empty).into_value(tag)
                    }
                })))
            } else {
                let mut obj = input.clone();

                for column in column_paths.clone() {
                    let path = UntaggedValue::Primitive(Primitive::ColumnPath(column.clone()))
                        .into_value(tag);
                    let data = r.get_data(&as_string(&path)?).borrow().clone();
                    let is_empty = data.is_empty();

                    let default = if default_block.is_some() {
                        if is_empty {
                            results
                                .clone()
                                .unwrap_or_else(|| UntaggedValue::boolean(true).into_value(tag))
                        } else {
                            data.clone()
                        }
                    } else {
                        UntaggedValue::boolean(is_empty).into_value(tag)
                    };

                    if let Ok(value) =
                        obj.swap_data_by_column_path(&column, Box::new(move |_| Ok(default)))
                    {
                        obj = value;
                    }
                }

                Ok(OutputStream::one(ReturnSuccess::value(obj)))
            }
        }
        other => Ok(OutputStream::one(ReturnSuccess::value({
            if other.is_empty() {
                results
                    .clone()
                    .unwrap_or_else(|| UntaggedValue::boolean(true).into_value(other.tag))
            } else {
                UntaggedValue::boolean(false).into_value(other.tag)
            }
        }))),
    }
}
