use indexmap::IndexMap;
use nu_engine::{command_prelude::*, get_eval_block_with_early_return};
use nu_protocol::engine::Closure;

#[derive(Clone)]
pub struct Rename;

impl Command for Rename {
    fn name(&self) -> &str {
        "rename"
    }

    fn signature(&self) -> Signature {
        Signature::build("rename")
            .input_output_types(vec![
                (Type::Record(vec![]), Type::Record(vec![])),
                (Type::Table(vec![]), Type::Table(vec![])),
            ])
            .named(
                "column",
                SyntaxShape::Record(vec![]),
                "column name to be changed",
                Some('c'),
            )
            .named(
                "block",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                "A closure to apply changes on each column",
                Some('b'),
            )
            .rest(
                "rest",
                SyntaxShape::String,
                "The new names for the columns.",
            )
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Creates a new table with columns renamed."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        rename(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Rename a column",
                example: "[[a, b]; [1, 2]] | rename my_column",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "my_column" => Value::test_int(1),
                    "b" =>         Value::test_int(2),
                })])),
            },
            Example {
                description: "Rename many columns",
                example: "[[a, b, c]; [1, 2, 3]] | rename eggs ham bacon",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "eggs" =>  Value::test_int(1),
                    "ham" =>   Value::test_int(2),
                    "bacon" => Value::test_int(3),
                })])),
            },
            Example {
                description: "Rename a specific column",
                example: "[[a, b, c]; [1, 2, 3]] | rename --column { a: ham }",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "ham" => Value::test_int(1),
                    "b" =>   Value::test_int(2),
                    "c" =>   Value::test_int(3),
                })])),
            },
            Example {
                description: "Rename the fields of a record",
                example: "{a: 1 b: 2} | rename x y",
                result: Some(Value::test_record(record! {
                    "x" => Value::test_int(1),
                    "y" => Value::test_int(2),
                })),
            },
            Example {
                description: "Rename fields based on a given closure",
                example: "{abc: 1, bbc: 2} | rename --block {str replace --all 'b' 'z'}",
                result: Some(Value::test_record(record! {
                    "azc" => Value::test_int(1),
                    "zzc" => Value::test_int(2),
                })),
            },
        ]
    }
}

fn rename(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let specified_column: Option<Record> = call.get_flag(engine_state, stack, "column")?;
    // convert from Record to HashMap for easily query.
    let specified_column: Option<IndexMap<String, String>> = match specified_column {
        Some(query) => {
            let mut columns = IndexMap::new();
            for (col, val) in query {
                let val_span = val.span();
                match val {
                    Value::String { val, .. } => {
                        columns.insert(col, val);
                    }
                    _ => {
                        return Err(ShellError::TypeMismatch {
                            err_message: "new column name must be a string".to_owned(),
                            span: val_span,
                        });
                    }
                }
            }
            if columns.is_empty() {
                return Err(ShellError::TypeMismatch {
                    err_message: "The column info cannot be empty".to_owned(),
                    span: call.head,
                });
            }
            Some(columns)
        }
        None => None,
    };
    let block_info =
        if let Some(capture_block) = call.get_flag::<Closure>(engine_state, stack, "block")? {
            let engine_state = engine_state.clone();
            let block = engine_state.get_block(capture_block.block_id).clone();
            let stack = stack.captures_to_stack(capture_block.captures);
            let orig_env_vars = stack.env_vars.clone();
            let orig_env_hidden = stack.env_hidden.clone();
            Some((engine_state, block, stack, orig_env_vars, orig_env_hidden))
        } else {
            None
        };

    let columns: Vec<String> = call.rest(engine_state, stack, 0)?;
    let metadata = input.metadata();

    let eval_block_with_early_return = get_eval_block_with_early_return(engine_state);

    let head_span = call.head;
    input
        .map(
            move |item| {
                let span = item.span();
                match item {
                    Value::Record { val: record, .. } => {
                        let record =
                            if let Some((engine_state, block, mut stack, env_vars, env_hidden)) =
                                block_info.clone()
                            {
                                record
                                    .into_owned().into_iter()
                                    .map(|(col, val)| {
                                        stack.with_env(&env_vars, &env_hidden);

                                        if let Some(var) = block.signature.get_positional(0) {
                                            if let Some(var_id) = &var.var_id {
                                                stack.add_var(
                                                    *var_id,
                                                    Value::string(col.clone(), span),
                                                )
                                            }
                                        }

                                        eval_block_with_early_return(
                                            &engine_state,
                                            &mut stack,
                                            &block,
                                            Value::string(col, span).into_pipeline_data(),
                                        )
                                        .and_then(|data| data.collect_string_strict(span))
                                        .map(|(col, _, _)| (col, val))
                                    })
                                    .collect::<Result<Record, _>>()
                            } else {
                                match &specified_column {
                                    Some(columns) => {
                                        // record columns are unique so we can track the number
                                        // of renamed columns to check if any were missed
                                        let mut renamed = 0;
                                        let record = record.into_owned().into_iter().map(|(col, val)| {
                                            let col = if let Some(col) = columns.get(&col) {
                                                renamed += 1;
                                                col.clone()
                                            } else {
                                                col
                                            };

                                            (col, val)
                                        }).collect::<Record>();

                                        let missing_column = if renamed < columns.len() {
                                            columns.iter().find_map(|(col, new_col)| {
                                                (!record.contains(new_col)).then_some(col)
                                            })
                                        } else {
                                            None
                                        };

                                        if let Some(missing) = missing_column {
                                            Err(ShellError::UnsupportedInput {
                                                msg: format!("The column '{missing}' does not exist in the input"),
                                                input: "value originated from here".into(),
                                                msg_span: head_span,
                                                input_span: span,
                                            })
                                        } else {
                                            Ok(record)
                                        }
                                    }
                                    None => Ok(record
                                        .into_owned().into_iter()
                                        .enumerate()
                                        .map(|(i, (col, val))| {
                                            (columns.get(i).cloned().unwrap_or(col), val)
                                        })
                                        .collect()),
                                }
                            };

                        match record {
                            Ok(record) => Value::record(record, span),
                            Err(err) => Value::error(err, span),
                        }
                    }
                    // Propagate errors by explicitly matching them before the final case.
                    Value::Error { .. } => item,
                    other => Value::error(
                        ShellError::OnlySupportsThisInputType {
                            exp_input_type: "record".into(),
                            wrong_type: other.get_type().to_string(),
                            dst_span: head_span,
                            src_span: other.span(),
                        },
                        head_span,
                    ),
                }
            },
            engine_state.ctrlc.clone(),
        )
        .map(|x| x.set_metadata(metadata))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Rename {})
    }
}
