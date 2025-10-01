use super::common::{MergeStrategy, do_merge, typecheck_merge};
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
                SyntaxShape::OneOf(vec![
                    SyntaxShape::Record(vec![]),
                    SyntaxShape::Table(vec![]),
                ]),
                "The new value to merge with.",
            )
            .category(Category::Filters)
    }

    fn examples(&self) -> Vec<Example<'_>> {
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

        // collect input before typechecking, so tables are detected as such
        let input_span = input.span().unwrap_or(head);
        let input = input.into_value(input_span)?;

        typecheck_merge(&input, &merge_value, head)?;

        let merged = do_merge(input, merge_value, MergeStrategy::Shallow, head)?;
        Ok(merged.into_pipeline_data_with_metadata(metadata))
    }
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
