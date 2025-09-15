use nu_engine::{column::get_columns, command_prelude::*};

#[derive(Clone)]
pub struct Transpose;

pub struct TransposeArgs {
    rest: Vec<Spanned<String>>,
    header_row: bool,
    ignore_titles: bool,
    as_record: bool,
    keep_last: bool,
    keep_all: bool,
}

impl Command for Transpose {
    fn name(&self) -> &str {
        "transpose"
    }

    fn signature(&self) -> Signature {
        Signature::build("transpose")
            .input_output_types(vec![
                (Type::table(), Type::Any),
                (Type::record(), Type::table()),
            ])
            .switch(
                "header-row",
                "use the first input column as the table header-row (or keynames when combined with --as-record)",
                Some('r'),
            )
            .switch(
                "ignore-titles",
                "don't transpose the column names into values",
                Some('i'),
            )
            .switch(
                "as-record",
                "transfer to record if the result is a table and contains only one row",
                Some('d'),
            )
            .switch(
                "keep-last",
                "on repetition of record fields due to `header-row`, keep the last value obtained",
                Some('l'),
            )
            .switch(
                "keep-all",
                "on repetition of record fields due to `header-row`, keep all the values obtained",
                Some('a'),
            )
            .allow_variants_without_examples(true)
            .rest(
                "rest",
                SyntaxShape::String,
                "The names to give columns once transposed.",
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Transposes the table contents so rows become columns and columns become rows."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["pivot"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        transpose(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Transposes the table contents with default column names",
                example: "[[c1 c2]; [1 2]] | transpose",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "column0" => Value::test_string("c1"),
                        "column1" => Value::test_int(1),
                    }),
                    Value::test_record(record! {
                        "column0" =>  Value::test_string("c2"),
                        "column1" =>  Value::test_int(2),
                    }),
                ])),
            },
            Example {
                description: "Transposes the table contents with specified column names",
                example: "[[c1 c2]; [1 2]] | transpose key val",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "key" =>  Value::test_string("c1"),
                        "val" =>  Value::test_int(1),
                    }),
                    Value::test_record(record! {
                        "key" =>  Value::test_string("c2"),
                        "val" =>  Value::test_int(2),
                    }),
                ])),
            },
            Example {
                description: "Transposes the table without column names and specify a new column name",
                example: "[[c1 c2]; [1 2]] | transpose --ignore-titles val",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "val" => Value::test_int(1),
                    }),
                    Value::test_record(record! {
                        "val" => Value::test_int(2),
                    }),
                ])),
            },
            Example {
                description: "Transfer back to record with -d flag",
                example: "{c1: 1, c2: 2} | transpose | transpose --ignore-titles -r -d",
                result: Some(Value::test_record(record! {
                    "c1" =>  Value::test_int(1),
                    "c2" =>  Value::test_int(2),
                })),
            },
        ]
    }
}

pub fn transpose(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let name = call.head;
    let args = TransposeArgs {
        header_row: call.has_flag(engine_state, stack, "header-row")?,
        ignore_titles: call.has_flag(engine_state, stack, "ignore-titles")?,
        as_record: call.has_flag(engine_state, stack, "as-record")?,
        keep_last: call.has_flag(engine_state, stack, "keep-last")?,
        keep_all: call.has_flag(engine_state, stack, "keep-all")?,
        rest: call.rest(engine_state, stack, 0)?,
    };

    if !args.rest.is_empty() && args.header_row {
        return Err(ShellError::IncompatibleParametersSingle {
            msg: "Can not provide header names and use `--header-row`".into(),
            span: call.get_flag_span(stack, "header-row").expect("has flag"),
        });
    }
    if !args.header_row && args.keep_all {
        return Err(ShellError::IncompatibleParametersSingle {
            msg: "Can only be used with `--header-row`(`-r`)".into(),
            span: call.get_flag_span(stack, "keep-all").expect("has flag"),
        });
    }
    if !args.header_row && args.keep_last {
        return Err(ShellError::IncompatibleParametersSingle {
            msg: "Can only be used with `--header-row`(`-r`)".into(),
            span: call.get_flag_span(stack, "keep-last").expect("has flag"),
        });
    }
    if args.keep_all && args.keep_last {
        return Err(ShellError::IncompatibleParameters {
            left_message: "can't use `--keep-last` at the same time".into(),
            left_span: call.get_flag_span(stack, "keep-last").expect("has flag"),
            right_message: "because of `--keep-all`".into(),
            right_span: call.get_flag_span(stack, "keep-all").expect("has flag"),
        });
    }

    let metadata = input.metadata();
    let input: Vec<_> = input.into_iter().collect();

    // Ensure error values are propagated and non-record values are rejected
    for value in input.iter() {
        match value {
            Value::Error { .. } => {
                return Ok(value.clone().into_pipeline_data_with_metadata(metadata));
            }
            Value::Record { .. } => {} // go on, this is what we're looking for
            _ => {
                return Err(ShellError::OnlySupportsThisInputType {
                    exp_input_type: "table or record".into(),
                    wrong_type: "list<any>".into(),
                    dst_span: call.head,
                    src_span: value.span(),
                });
            }
        }
    }

    let descs = get_columns(&input);

    let mut headers: Vec<String> = Vec::with_capacity(input.len());

    if args.header_row {
        for i in input.iter() {
            if let Some(desc) = descs.first() {
                match &i.get_data_by_key(desc) {
                    Some(x) => {
                        if let Ok(s) = x.coerce_string() {
                            headers.push(s);
                        } else {
                            return Err(ShellError::GenericError {
                                error: "Header row needs string headers".into(),
                                msg: "used non-string headers".into(),
                                span: Some(name),
                                help: None,
                                inner: vec![],
                            });
                        }
                    }
                    _ => {
                        return Err(ShellError::GenericError {
                            error: "Header row is incomplete and can't be used".into(),
                            msg: "using incomplete header row".into(),
                            span: Some(name),
                            help: None,
                            inner: vec![],
                        });
                    }
                }
            } else {
                return Err(ShellError::GenericError {
                    error: "Header row is incomplete and can't be used".into(),
                    msg: "using incomplete header row".into(),
                    span: Some(name),
                    help: None,
                    inner: vec![],
                });
            }
        }
    } else {
        for i in 0..=input.len() {
            if let Some(name) = args.rest.get(i) {
                headers.push(name.item.clone())
            } else {
                headers.push(format!("column{i}"));
            }
        }
    }

    let mut descs = descs.into_iter();
    if args.header_row {
        descs.next();
    }
    let mut result_data = descs
        .map(|desc| {
            let mut column_num: usize = 0;
            let mut record = Record::new();

            if !args.ignore_titles && !args.header_row {
                record.push(
                    headers[column_num].clone(),
                    Value::string(desc.clone(), name),
                );
                column_num += 1
            }

            for i in input.iter() {
                let x = i
                    .get_data_by_key(&desc)
                    .unwrap_or_else(|| Value::nothing(name));
                match record.get_mut(&headers[column_num]) {
                    None => {
                        record.push(headers[column_num].clone(), x);
                    }
                    Some(val) => {
                        if args.keep_all {
                            let current_span = val.span();
                            match val {
                                Value::List { vals, .. } => {
                                    vals.push(x);
                                }
                                v => {
                                    *v = Value::list(vec![std::mem::take(v), x], current_span);
                                }
                            };
                        } else if args.keep_last {
                            *val = x;
                        }
                    }
                }

                column_num += 1;
            }

            Value::record(record, name)
        })
        .collect::<Vec<Value>>();
    if result_data.len() == 1 && args.as_record {
        Ok(PipelineData::value(
            result_data
                .pop()
                .expect("already check result only contains one item"),
            metadata,
        ))
    } else {
        Ok(result_data.into_pipeline_data_with_metadata(
            name,
            engine_state.signals().clone(),
            metadata,
        ))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Transpose {})
    }
}
