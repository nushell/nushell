use super::common::{ListMerge, MergeStrategy, do_merge, typecheck_merge};
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
        r#"The way that key-value pairs which exist in both the input and the argument are merged depends on their types.

Scalar values (like numbers and strings) in the input are overwritten by the corresponding value from the argument.
Records in the input are merged similarly to the merge command, but recursing rather than overwriting inner records.

The way lists and tables are merged is controlled by the `--strategy` flag:
  - table: Merges tables element-wise, similarly to the merge command. Non-table lists are overwritten.
  - overwrite: Lists and tables are overwritten with their corresponding value from the argument, similarly to scalars.
  - append: Lists and tables in the input are appended with the corresponding list from the argument.
  - prepend: Lists and tables in the input are prepended with the corresponding list from the argument."#
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("merge deep")
            .input_output_types(vec![
                (Type::record(), Type::record()),
                (Type::table(), Type::table()),
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
            .param(
                Flag::new("strategy")
                    .short('s')
                    .arg(SyntaxShape::String)
                    .desc(
                        "The list merging strategy to use. One of: table (default), overwrite, \
                         append, prepend",
                    )
                    .completion(Completion::new_list(&[
                        "table",
                        "overwrite",
                        "append",
                        "prepend",
                    ])),
            )
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: "{a: 1, b: {c: 2, d: 3}} | merge deep {b: {d: 4, e: 5}}",
                description: "Merge two records recursively",
                result: Some(Value::test_record(record! {
                    "a" => Value::test_int(1),
                    "b" => Value::test_record(record! {
                        "c" => Value::test_int(2),
                        "d" => Value::test_int(4),
                        "e" => Value::test_int(5),
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

        let merged = do_merge(input, merge_value, strategy, head)?;
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
