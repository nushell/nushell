use super::{HorizontalDirection, horizontal_rotate_value};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct RollRight;

impl Command for RollRight {
    fn name(&self) -> &str {
        "roll right"
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
        "Roll table columns right."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Rolls columns of a record to the right",
                example: "{a:1 b:2 c:3} | roll right",
                result: Some(Value::test_record(record! {
                    "c" => Value::test_int(3),
                    "a" => Value::test_int(1),
                    "b" => Value::test_int(2),
                })),
            },
            Example {
                description: "Rolls columns to the right",
                example: "[[a b c]; [1 2 3] [4 5 6]] | roll right",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "c" => Value::test_int(3),
                        "a" => Value::test_int(1),
                        "b" => Value::test_int(2),
                    }),
                    Value::test_record(record! {
                        "c" => Value::test_int(6),
                        "a" => Value::test_int(4),
                        "b" => Value::test_int(5),
                    }),
                ])),
            },
            Example {
                description: "Rolls columns to the right with fixed headers",
                example: "[[a b c]; [1 2 3] [4 5 6]] | roll right --cells-only",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "a" => Value::test_int(3),
                        "b" => Value::test_int(1),
                        "c" => Value::test_int(2),
                    }),
                    Value::test_record(record! {
                        "a" => Value::test_int(6),
                        "b" => Value::test_int(4),
                        "c" => Value::test_int(5),
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
            horizontal_rotate_value(value, by, cells_only, &HorizontalDirection::Right)?;

        Ok(rotated_value.into_pipeline_data().set_metadata(metadata))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(RollRight {})
    }
}
