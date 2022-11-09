use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape,
    Type, Value,
};

use super::{horizontal_rotate_value, HorizontalDirection};

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
            .input_output_types(vec![(Type::Table(vec![]), Type::Table(vec![]))])
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

    fn usage(&self) -> &str {
        "Roll table columns left"
    }

    fn examples(&self) -> Vec<Example> {
        let columns = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let rotated_columns = vec!["b".to_string(), "c".to_string(), "a".to_string()];
        vec![
            Example {
                description: "Rolls columns to the left",
                example: "[[a b c]; [1 2 3] [4 5 6]] | roll left",
                result: Some(Value::List {
                    vals: vec![
                        Value::Record {
                            cols: rotated_columns.clone(),
                            vals: vec![Value::test_int(2), Value::test_int(3), Value::test_int(1)],
                            span: Span::test_data(),
                        },
                        Value::Record {
                            cols: rotated_columns,
                            vals: vec![Value::test_int(5), Value::test_int(6), Value::test_int(4)],
                            span: Span::test_data(),
                        },
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Rolls columns to the left with fixed headers",
                example: "[[a b c]; [1 2 3] [4 5 6]] | roll left --cells-only",
                result: Some(Value::List {
                    vals: vec![
                        Value::Record {
                            cols: columns.clone(),
                            vals: vec![Value::test_int(2), Value::test_int(3), Value::test_int(1)],
                            span: Span::test_data(),
                        },
                        Value::Record {
                            cols: columns,
                            vals: vec![Value::test_int(5), Value::test_int(6), Value::test_int(4)],
                            span: Span::test_data(),
                        },
                    ],
                    span: Span::test_data(),
                }),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, ShellError> {
        let by: Option<usize> = call.get_flag(engine_state, stack, "by")?;
        let metadata = input.metadata();

        let cells_only = call.has_flag("cells-only");
        let value = input.into_value(call.head);
        let rotated_value =
            horizontal_rotate_value(value, &by, cells_only, &HorizontalDirection::Left)?;

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
