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
            .switch("deep", "Perform a deep merge", Some('d'))
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
            Example {
                example: "{a: {foo: 123}, b: 2} | merge --deep {a: {bar: 456}}",
                description:
                    "Deep merge two records, combining inner records instead of overwriting",
                result: Some(Value::test_record(record! {
                    "a" => Value::test_record(record! {
                        "foo" => Value::test_int(123),
                        "bar" => Value::test_int(456),
                    }),
                    "b" => Value::test_int(2)
                })),
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
        let deep = call.has_flag(engine_state, stack, "deep")?;
        let metadata = input.metadata();

        match (input, merge_value) {
            // table (list of records)
            (
                input @ (PipelineData::Value(Value::List { .. }, ..)
                | PipelineData::ListStream { .. }),
                Value::List { vals, .. },
            ) => {
                let mut table_iter = vals.into_iter();

                let res = input.into_iter().map(move |inp| {
                    match (inp.into_record(), table_iter.next()) {
                        (Ok(rec), Some(to_merge)) => match to_merge.into_record() {
                            Ok(to_merge) => Value::record(
                                do_merge(rec.to_owned(), to_merge.to_owned(), head, deep),
                                head,
                            ),
                            Err(error) => Value::error(error, head),
                        },
                        (Ok(rec), None) => Value::record(rec, head),
                        (Err(error), _) => Value::error(error, head),
                    }
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
            ) => Ok(Value::record(
                do_merge(inp.into_owned(), to_merge.into_owned(), head, deep),
                head,
            )
            .into_pipeline_data()),
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

fn do_merge(mut source: Record, merge: Record, span: Span, deep: bool) -> Record {
    for (col, val) in merge {
        // in order to both avoid cloning (possibly nested) record values and maintain the ordering of record keys, we can swap a temporary value into the source record.
        // if we were to remove the value, the ordering would be messed up as we might not insert back into the original index
        // it's okay to swap a temporary value in, since we know it will be replaced by the end of the function call
        //
        // use an error here instead of something like null so if this somehow makes it into the output, the bug will be immediately obvious
        let failed_error = ShellError::NushellFailed {
            msg: "Merge failed to properly replace internal temporary value".to_owned(),
        };
        let value = match (
            deep,
            source.insert(&col, Value::error(failed_error, span)),
            val,
        ) {
            (
                true,
                Some(Value::Record { val: inner_src, .. }),
                Value::Record {
                    val: inner_merge, ..
                },
            ) => Value::record(
                do_merge(inner_src.into_owned(), inner_merge.into_owned(), span, deep),
                span,
            ),
            (
                true,
                Some(Value::List {
                    vals: inner_src, ..
                }),
                Value::List {
                    vals: inner_merge, ..
                },
            ) => Value::list(
                inner_src
                    .into_iter()
                    .chain(inner_merge.into_iter())
                    .collect(),
                span,
            ),
            (_, _, val) => val,
        };
        source.insert(col, value);
    }
    source
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
