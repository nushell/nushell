use nu_engine::{command_prelude::*, ClosureEval, ClosureEvalOnce};
use nu_protocol::engine::Closure;

#[derive(Clone)]
pub struct EachWhile;

impl Command for EachWhile {
    fn name(&self) -> &str {
        "each while"
    }

    fn usage(&self) -> &str {
        "Run a block on each row of the input list until a null is found, then create a new list with the results."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["for", "loop", "iterate"]
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build(self.name())
            .input_output_types(vec![(
                Type::List(Box::new(Type::Any)),
                Type::List(Box::new(Type::Any)),
            )])
            .required(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any, SyntaxShape::Int])),
                "the closure to run",
            )
            .category(Category::Filters)
    }

    fn examples(&self) -> Vec<Example> {
        let stream_test_1 = vec![Value::test_int(2), Value::test_int(4)];
        let stream_test_2 = vec![
            Value::test_string("Output: 1"),
            Value::test_string("Output: 2"),
        ];

        vec![
            Example {
                example: "[1 2 3 2 1] | each while {|e| if $e < 3 { $e * 2 } }",
                description: "Produces a list of each element before the 3, doubled",
                result: Some(Value::list(stream_test_1, Span::test_data())),
            },
            Example {
                example: r#"[1 2 stop 3 4] | each while {|e| if $e != 'stop' { $"Output: ($e)" } }"#,
                description: "Output elements until reaching 'stop'",
                result: Some(Value::list(stream_test_2, Span::test_data())),
            },
            Example {
                example: r#"[1 2 3] | enumerate | each while {|e| if $e.item < 2 { $"value ($e.item) at ($e.index)!"} }"#,
                description: "Iterate over each element, printing the matching value and its index",
                result: Some(Value::list(
                    vec![Value::test_string("value 1 at 0!")],
                    Span::test_data(),
                )),
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
        let closure: Closure = call.req(engine_state, stack, 0)?;

        let metadata = input.metadata();
        match input {
            PipelineData::Empty => Ok(PipelineData::Empty),
            PipelineData::Value(Value::Range { .. }, ..)
            | PipelineData::Value(Value::List { .. }, ..)
            | PipelineData::ListStream(..) => {
                let mut closure = ClosureEval::new(engine_state, stack, closure);
                Ok(input
                    .into_iter()
                    .map_while(move |value| match closure.run_with_value(value) {
                        Ok(data) => {
                            let value = data.into_value(head);
                            (!value.is_nothing()).then_some(value)
                        }
                        Err(_) => None,
                    })
                    .fuse()
                    .into_pipeline_data(engine_state.ctrlc.clone()))
            }
            PipelineData::ExternalStream { stdout: None, .. } => Ok(PipelineData::empty()),
            PipelineData::ExternalStream {
                stdout: Some(stream),
                ..
            } => {
                let mut closure = ClosureEval::new(engine_state, stack, closure);
                Ok(stream
                    .into_iter()
                    .map_while(move |value| {
                        let value = value.ok()?;
                        match closure.run_with_value(value) {
                            Ok(data) => {
                                let value = data.into_value(head);
                                (!value.is_nothing()).then_some(value)
                            }
                            Err(_) => None,
                        }
                    })
                    .fuse()
                    .into_pipeline_data(engine_state.ctrlc.clone()))
            }
            // This match allows non-iterables to be accepted,
            // which is currently considered undesirable (Nov 2022).
            PipelineData::Value(value, ..) => {
                ClosureEvalOnce::new(engine_state, stack, closure).run_with_value(value)
            }
        }
        .map(|data| data.set_metadata(metadata))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(EachWhile {})
    }
}
