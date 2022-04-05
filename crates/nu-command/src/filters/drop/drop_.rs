use nu_engine::CallExt;

use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, PipelineData, ShellError, Signature, Span,
    SyntaxShape, Value,
};

#[derive(Clone)]
pub struct Drop;

impl Command for Drop {
    fn name(&self) -> &str {
        "drop"
    }

    fn signature(&self) -> Signature {
        Signature::build("drop")
            .optional(
                "rows",
                SyntaxShape::Int,
                "starting from the back, the number of rows to remove",
            )
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Remove the last number of rows or columns."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["delete"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "[0,1,2,3] | drop",
                description: "Remove the last item of a list/table",
                result: Some(Value::List {
                    vals: vec![Value::test_int(0), Value::test_int(1), Value::test_int(2)],
                    span: Span::test_data(),
                }),
            },
            Example {
                example: "[0,1,2,3] | drop 0",
                description: "Remove zero item of a list/table",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_int(0),
                        Value::test_int(1),
                        Value::test_int(2),
                        Value::test_int(3),
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                example: "[0,1,2,3] | drop 2",
                description: "Remove the last two items of a list/table",
                result: Some(Value::List {
                    vals: vec![Value::test_int(0), Value::test_int(1)],
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
    ) -> Result<PipelineData, ShellError> {
        let metadata = input.metadata();
        let rows: Option<i64> = call.opt(engine_state, stack, 0)?;
        let v: Vec<_> = input.into_iter().collect();
        let vlen: i64 = v.len() as i64;

        let rows_to_drop = if let Some(quantity) = rows {
            quantity
        } else {
            1
        };

        if rows_to_drop == 0 {
            Ok(v.into_iter()
                .into_pipeline_data(engine_state.ctrlc.clone())
                .set_metadata(metadata))
        } else {
            let k = if vlen < rows_to_drop {
                0
            } else {
                vlen - rows_to_drop
            };

            let iter = v.into_iter().take(k as usize);
            Ok(iter
                .into_pipeline_data(engine_state.ctrlc.clone())
                .set_metadata(metadata))
        }
    }
}

#[cfg(test)]
mod test {
    use crate::Drop;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Drop {})
    }
}
