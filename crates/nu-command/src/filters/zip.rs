use nu_engine::{get_eval_block_with_early_return, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData, ShellError,
    Signature, Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct Zip;

impl Command for Zip {
    fn name(&self) -> &str {
        "zip"
    }

    fn usage(&self) -> &str {
        "Combine a stream with the input."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("zip")
            .input_output_types(vec![
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::List(Box::new(Type::Any)))),
                ),
                (
                    Type::Range,
                    Type::List(Box::new(Type::List(Box::new(Type::Any)))),
                ),
            ])
            .required(
                "other",
                SyntaxShape::OneOf(vec![SyntaxShape::Any, SyntaxShape::Closure(Some(vec![]))]),
                "The other input, or closure returning a stream.",
            )
            .category(Category::Filters)
    }

    fn examples(&self) -> Vec<Example> {
        let test_row_1 = Value::list(
            vec![Value::test_int(1), Value::test_int(4)],
            Span::test_data(),
        );

        let test_row_2 = Value::list(
            vec![Value::test_int(2), Value::test_int(5)],
            Span::test_data(),
        );

        let test_row_3 = Value::list(
            vec![Value::test_int(3), Value::test_int(6)],
            Span::test_data(),
        );

        vec![
            Example {
                example: "[1 2] | zip [3 4]",
                description: "Zip two lists",
                result: Some(Value::list(
                    vec![
                        Value::list(
                            vec![Value::test_int(1), Value::test_int(3)],
                            Span::test_data(),
                        ),
                        Value::list(
                            vec![Value::test_int(2), Value::test_int(4)],
                            Span::test_data(),
                        ),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                example: "1..3 | zip 4..6",
                description: "Zip two ranges",
                result: Some(Value::list(
                    vec![test_row_1.clone(), test_row_2.clone(), test_row_3.clone()],
                    Span::test_data(),
                )),
            },
            Example {
                example: "seq 1 3 | zip { seq 4 600000000 }",
                description: "Zip two streams",
                result: Some(Value::list(
                    vec![test_row_1, test_row_2, test_row_3],
                    Span::test_data(),
                )),
            },
            Example {
                example: "glob *.ogg | zip ['bang.ogg', 'fanfare.ogg', 'laser.ogg'] | each {|| mv $in.0 $in.1 }",
                description: "Rename .ogg files to match an existing list of filenames",
                result: None,
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
        let ctrlc = engine_state.ctrlc.clone();
        let metadata = input.metadata();
        let eval_block_with_early_return = get_eval_block_with_early_return(engine_state);

        let other: PipelineData = match call.req(engine_state, stack, 0)? {
            // If a closure was provided, evaluate it and consume its stream output
            Value::Closure { val, .. } => {
                let block = engine_state.get_block(val.block_id);
                let mut stack = stack.captures_to_stack(val.captures);
                eval_block_with_early_return(engine_state, &mut stack, block, PipelineData::Empty)?
            }
            // If any other value, use it as-is.
            val => val.into_pipeline_data(),
        };

        Ok(input
            .into_iter()
            .zip(other)
            .map(move |(x, y)| Value::list(vec![x, y], head))
            .into_pipeline_data_with_metadata(metadata, ctrlc))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Zip {})
    }
}
