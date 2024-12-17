use super::common::{do_merge, typecheck_merge, ListMerge, MergeStrategy};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct MergeDeep;

impl Command for MergeDeep {
    fn name(&self) -> &str {
        "merge deep"
    }

    fn description(&self) -> &str {
        "Merge the input with a record or table, recursively merging values in matching columns."
    }

    fn extra_description(&self) -> &str {
        r#"How values are merged depends on their types.
  - For scalar values like ints and strings, value from the argument simply
    overwrites value from the input.
  - For records, merging is similar to `merge`, happening recursively.
  - For lists and tables, it is controlled by the `--strategy` flag."#
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("merge deep")
            .input_output_types(vec![
                (Type::record(), Type::record()),
                (Type::table(), Type::table()),
                // actually a non-table list of records, but there is no way to express this
                (Type::list(Type::Any), Type::list(Type::Any)),
            ])
            .required(
                "value",
                SyntaxShape::OneOf(vec![
                    SyntaxShape::Record(vec![]),
                    SyntaxShape::Table(vec![]),
                    SyntaxShape::List(SyntaxShape::Any.into()),
                ]),
                "The new value to merge with.",
            )
            .category(Category::Filters)
            .named("strategy", SyntaxShape::String, "The list merging strategy to use. One of: table (default), overwrite, append, prepend", Some('s'))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "{a: 1, b: {c: 2}} | merge deep {b: {c: 3, d: 4}}",
                description: "Merge two records",
                result: Some(Value::test_record(record! {
                    "a" => Value::test_int(1),
                    "b" => Value::test_record(record! {
                        "c" => Value::test_int(3),
                        "d" => Value::test_int(4),
                    })
                })),
            },
            Example {
                example: r#"[{columnA: 0, columnB: [{B1: 1}]}] | merge deep [{columnB: [{B2: 2}]}]"#,
                description: "Merge two tables",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "columnA" => Value::test_int(0),
                    "columnB" => Value::test_list(vec![
                        Value::test_record(record! {
                            "B1" => Value::test_int(1),
                            "B2" => Value::test_int(2),
                        })
                    ]),
                })])),
            },
            Example {
                example: r#"{inner: [{a: 1}, {b: 2}]} | merge deep {inner: [{c: 3}]}"#,
                description: "Merge two records and their inner tables",
                result: Some(Value::test_record(record! {
                    "inner" => Value::test_list(vec![
                        Value::test_record(record! {
                            "a" => Value::test_int(1),
                            "c" => Value::test_int(3),
                        }),
                        Value::test_record(record! {
                            "b" => Value::test_int(2),
                        })
                    ])
                })),
            },
            Example {
                example: r#"{inner: [{a: 1}, {b: 2}]} | merge deep {inner: [{c: 3}]} --strategy=append"#,
                description: "Merge two records, appending their inner tables",
                result: Some(Value::test_record(record! {
                    "inner" => Value::test_list(vec![
                        Value::test_record(record! {
                            "a" => Value::test_int(1),
                        }),
                        Value::test_record(record! {
                            "b" => Value::test_int(2),
                        }),
                        Value::test_record(record! {
                            "c" => Value::test_int(3),
                        }),
                    ])
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
        let strategy_flag: Option<String> = call.get_flag(engine_state, stack, "strategy")?;
        let metadata = input.metadata();

        // collect input before typechecking, so tables are detected as such
        let input_span = input.span().unwrap_or(head);
        let input = input.into_value(input_span)?;

        let strategy = match strategy_flag.as_deref() {
            None | Some("table") => MergeStrategy::Deep(ListMerge::Elementwise),
            Some("append") => MergeStrategy::Deep(ListMerge::Append),
            Some("prepend") => MergeStrategy::Deep(ListMerge::Prepend),
            Some("overwrite") => MergeStrategy::Deep(ListMerge::Overwrite),
            Some(_) => {
                return Err(ShellError::IncorrectValue {
                    msg: "The list merging strategy must be one one of: table, overwrite, append, prepend".to_string(),
                    val_span: call.get_flag_span(stack, "strategy").unwrap_or(head),
                    call_span: head,
                })
            }
        };

        typecheck_merge(&input, &merge_value, head)?;

        let merged = do_merge(input, merge_value, strategy, head, true)?;
        Ok(merged.into_pipeline_data_with_metadata(metadata))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(MergeDeep {})
    }
}
