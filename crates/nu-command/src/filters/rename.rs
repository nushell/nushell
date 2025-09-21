use indexmap::IndexMap;
use nu_engine::{ClosureEval, command_prelude::*};
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
                (Type::record(), Type::record()),
                (Type::table(), Type::table()),
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

    fn description(&self) -> &str {
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

    fn examples(&self) -> Vec<Example<'_>> {
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
    let head = call.head;
    let columns: Vec<String> = call.rest(engine_state, stack, 0)?;
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
    let closure: Option<Closure> = call.get_flag(engine_state, stack, "block")?;

    let mut closure = closure.map(|closure| ClosureEval::new(engine_state, stack, closure));

    let metadata = input.metadata();
    input
        .map(
            move |item| {
                let span = item.span();
                match item {
                    Value::Record { val: record, .. } => {
                        let record = if let Some(closure) = &mut closure {
                            record
                                .into_owned()
                                .into_iter()
                                .map(|(col, val)| {
                                    let col = Value::string(col, span);
                                    let data = closure.run_with_value(col)?;
                                    let col = data.collect_string_strict(span)?.0;
                                    Ok((col, val))
                                })
                                .collect::<Result<Record, _>>()
                        } else {
                            match &specified_column {
                                Some(columns) => {
                                    // record columns are unique so we can track the number
                                    // of renamed columns to check if any were missed
                                    let mut renamed = 0;
                                    let record = record
                                        .into_owned()
                                        .into_iter()
                                        .map(|(col, val)| {
                                            let col = if let Some(col) = columns.get(&col) {
                                                renamed += 1;
                                                col.clone()
                                            } else {
                                                col
                                            };

                                            (col, val)
                                        })
                                        .collect::<Record>();

                                    let missing_column = if renamed < columns.len() {
                                        columns.iter().find_map(|(col, new_col)| {
                                            (!record.contains(new_col)).then_some(col)
                                        })
                                    } else {
                                        None
                                    };

                                    if let Some(missing) = missing_column {
                                        Err(ShellError::UnsupportedInput {
                                            msg: format!(
                                                "The column '{missing}' does not exist in the input"
                                            ),
                                            input: "value originated from here".into(),
                                            msg_span: head,
                                            input_span: span,
                                        })
                                    } else {
                                        Ok(record)
                                    }
                                }
                                None => Ok(record
                                    .into_owned()
                                    .into_iter()
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
                            dst_span: head,
                            src_span: other.span(),
                        },
                        head,
                    ),
                }
            },
            engine_state.signals(),
        )
        .map(|data| data.set_metadata(metadata))
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
