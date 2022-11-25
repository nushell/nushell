use std::collections::VecDeque;

use nu_engine::CallExt;

use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData, ShellError,
    Signature, Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct Last;

impl Command for Last {
    fn name(&self) -> &str {
        "last"
    }

    fn signature(&self) -> Signature {
        Signature::build("last")
            .input_output_types(vec![
                (
                    // TODO: This variant duplicates the functionality of
                    // `take`. See #6611, #6611, #6893
                    // TODO: This is too permissive; if we could express this
                    // using a type parameter style it would be List<T> ->
                    // List<T>.
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Any)),
                ),
                (
                    // TODO: This is too permissive; if we could express this
                    // using a type parameter it would be List<T> -> T.
                    Type::List(Box::new(Type::Any)),
                    Type::Any,
                ),
            ])
            .optional(
                "rows",
                SyntaxShape::Int,
                "starting from the back, the number of rows to return",
            )
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Return only the last several rows of the input. Counterpart of 'first'. Opposite of 'drop'."
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "[1,2,3] | last 2",
                description: "Get the last 2 items",
                result: Some(Value::List {
                    vals: vec![Value::test_int(2), Value::test_int(3)],
                    span: Span::test_data(),
                }),
            },
            Example {
                example: "[1,2,3] | last",
                description: "Get the last item",
                result: Some(Value::test_int(3)),
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
        let span = call.head;

        let rows: Option<i64> = call.opt(engine_state, stack, 0)?;
        let to_keep = match rows.unwrap_or(1) {
            0 => {
                // early exit for `last 0`
                return Ok(Vec::<Value>::new()
                    .into_pipeline_data(engine_state.ctrlc.clone())
                    .set_metadata(metadata));
            }
            i if i < 0 => {
                return Err(ShellError::NeedsPositiveValue(span));
            }
            i => i as usize,
        };

        // only keep last `to_keep` rows in memory
        let mut buf = VecDeque::<_>::new();
        for row in input.into_iter() {
            if buf.len() == to_keep {
                buf.pop_front();
            }

            buf.push_back(row);
        }

        if rows.is_some() {
            Ok(buf
                .into_pipeline_data(engine_state.ctrlc.clone())
                .set_metadata(metadata))
        } else {
            let last = buf.pop_back();

            if let Some(last) = last {
                Ok(last.into_pipeline_data().set_metadata(metadata))
            } else {
                Ok(PipelineData::new(span).set_metadata(metadata))
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Last {})
    }
}
