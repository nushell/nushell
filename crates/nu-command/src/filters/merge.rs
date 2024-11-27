use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct Merge;

impl Command for Merge {
    fn name(&self) -> &str {
        "merge"
    }

    fn description(&self) -> &str {
        "Merge the input with a record or table, overwriting values in matching columns."
    }

    fn extra_description(&self) -> &str {
        r#"You may provide a column structure to merge

When merging tables, row 0 of the input table is overwritten
with values from row 0 of the provided table, then
repeating this process with row 1, and so on."#
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("merge")
            .input_output_types(vec![
                (Type::record(), Type::record()),
                (Type::table(), Type::table()),
            ])
            .required(
                "value",
                // Both this and `update` should have a shape more like <record> | <table> than just <any>. -Leon 2022-10-27
                SyntaxShape::Any,
                "The new value to merge with.",
            )
            .category(Category::Filters)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "[a b c] | wrap name | merge ( [47 512 618] | wrap id )",
                description: "Add an 'id' column to the input table",
                result: Some(Value::list(
                    vec![
                        Value::test_record(record! {
                            "name" => Value::test_string("a"),
                            "id" => Value::test_int(47),
                        }),
                        Value::test_record(record! {
                            "name" => Value::test_string("b"),
                            "id" => Value::test_int(512),
                        }),
                        Value::test_record(record! {
                            "name" => Value::test_string("c"),
                            "id" => Value::test_int(618),
                        }),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                example: "{a: 1, b: 2} | merge {c: 3}",
                description: "Merge two records",
                result: Some(Value::test_record(record! {
                    "a" => Value::test_int(1),
                    "b" => Value::test_int(2),
                    "c" => Value::test_int(3),
                })),
            },
            Example {
                example: "[{columnA: A0 columnB: B0}] | merge [{columnA: 'A0*'}]",
                description: "Merge two tables, overwriting overlapping columns",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "columnA" => Value::test_string("A0*"),
                    "columnB" => Value::test_string("B0"),
                })])),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let merge_value: Value = call.req(engine_state, stack, 0)?;
        let metadata = input.metadata();

        match (&input, merge_value) {
            // table (list of records)
            (
                PipelineData::Value(Value::List { .. }, ..) | PipelineData::ListStream { .. },
                Value::List { vals, .. },
            ) => {
                let mut table_iter = vals.into_iter();

                let res =
                    input
                        .into_iter()
                        .map(move |inp| match (inp.as_record(), table_iter.next()) {
                            (Ok(inp), Some(to_merge)) => match to_merge.as_record() {
                                Ok(to_merge) => Value::record(do_merge(inp, to_merge), head),
                                Err(error) => Value::error(error, head),
                            },
                            (_, None) => inp,
                            (Err(error), _) => Value::error(error, head),
                        });

                Ok(res.into_pipeline_data_with_metadata(
                    head,
                    engine_state.signals().clone(),
                    metadata,
                ))
            }
            // record
            (
                PipelineData::Value(Value::Record { val: inp, .. }, ..),
                Value::Record { val: to_merge, .. },
            ) => Ok(Value::record(do_merge(inp, &to_merge), head).into_pipeline_data()),
            // Propagate errors in the pipeline
            (PipelineData::Value(Value::Error { error, .. }, ..), _) => Err(*error.clone()),
            (PipelineData::Value(val, ..), ..) => {
                // Only point the "value originates here" arrow at the merge value
                // if it was generated from a block. Otherwise, point at the pipeline value. -Leon 2022-10-27
                let span = if val.span() == Span::test_data() {
                    Span::new(head.start, head.start)
                } else {
                    val.span()
                };

                Err(ShellError::PipelineMismatch {
                    exp_input_type: "input, and argument, to be both record or both table"
                        .to_string(),
                    dst_span: head,
                    src_span: span,
                })
            }
            _ => Err(ShellError::PipelineMismatch {
                exp_input_type: "input, and argument, to be both record or both table".to_string(),
                dst_span: head,
                src_span: Span::new(head.start, head.start),
            }),
        }
    }
}

// TODO: rewrite to mutate the input record
fn do_merge(input_record: &Record, to_merge_record: &Record) -> Record {
    let mut result = input_record.clone();

    for (col, val) in to_merge_record {
        result.insert(col, val.clone());
    }
    result
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Merge {})
    }
}
