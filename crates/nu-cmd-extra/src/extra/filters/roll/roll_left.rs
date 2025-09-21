use super::{HorizontalDirection, horizontal_rotate_value};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct RollLeft;

impl Command for RollLeft {
    fn name(&self) -> &str {
        "roll left"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["rotate", "shift", "move", "column"]
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![
                (Type::record(), Type::record()),
                (Type::table(), Type::table()),
            ])
            .named(
                "by",
                SyntaxShape::Int,
                "Number of columns to roll",
                Some('b'),
            )
            .switch(
                "cells-only",
                "rotates columns leaving headers fixed",
                Some('c'),
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Roll record or table columns left."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Rolls columns of a record to the left",
                example: "{a:1 b:2 c:3} | roll left",
                result: Some(Value::test_record(record! {
                    "b" => Value::test_int(2),
                    "c" => Value::test_int(3),
                    "a" => Value::test_int(1),
                })),
            },
            Example {
                description: "Rolls columns of a table to the left",
                example: "[[a b c]; [1 2 3] [4 5 6]] | roll left",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "b" => Value::test_int(2),
                        "c" => Value::test_int(3),
                        "a" => Value::test_int(1),
                    }),
                    Value::test_record(record! {
                        "b" => Value::test_int(5),
                        "c" => Value::test_int(6),
                        "a" => Value::test_int(4),
                    }),
                ])),
            },
            Example {
                description: "Rolls columns to the left without changing column names",
                example: "[[a b c]; [1 2 3] [4 5 6]] | roll left --cells-only",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "a" => Value::test_int(2),
                        "b" => Value::test_int(3),
                        "c" => Value::test_int(1),
                    }),
                    Value::test_record(record! {
                        "a" => Value::test_int(5),
                        "b" => Value::test_int(6),
                        "c" => Value::test_int(4),
                    }),
                ])),
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
        let by: Option<usize> = call.get_flag(engine_state, stack, "by")?;
        let metadata = input.metadata();

        let cells_only = call.has_flag(engine_state, stack, "cells-only")?;
        let value = input.into_value(call.head)?;
        let rotated_value =
            horizontal_rotate_value(value, by, cells_only, &HorizontalDirection::Left)?;

        Ok(rotated_value.into_pipeline_data().set_metadata(metadata))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(RollLeft {})
    }
}
