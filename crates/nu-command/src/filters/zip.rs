use nu_engine::{ClosureEvalOnce, command_prelude::*};

#[derive(Clone)]
pub struct Zip;

impl Command for Zip {
    fn name(&self) -> &str {
        "zip"
    }

    fn description(&self) -> &str {
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

    fn examples(&self) -> Vec<Example<'_>> {
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
        let other = call.req(engine_state, stack, 0)?;

        let metadata = input.metadata();
        let other = if let Value::Closure { val, .. } = other {
            // If a closure was provided, evaluate it and consume its stream output
            ClosureEvalOnce::new(engine_state, stack, *val).run_with_input(PipelineData::empty())?
        } else {
            other.into_pipeline_data()
        };

        Ok(input
            .into_iter()
            .zip(other)
            .map(move |(x, y)| Value::list(vec![x, y], head))
            .into_pipeline_data_with_metadata(head, engine_state.signals().clone(), metadata))
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
