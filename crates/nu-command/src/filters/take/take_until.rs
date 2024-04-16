use nu_engine::{command_prelude::*, ClosureEval};
use nu_protocol::engine::Closure;

#[derive(Clone)]
pub struct TakeUntil;

impl Command for TakeUntil {
    fn name(&self) -> &str {
        "take until"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![
                (Type::Table(vec![]), Type::Table(vec![])),
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Any)),
                ),
            ])
            .required(
                "predicate",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any, SyntaxShape::Int])),
                "The predicate that element(s) must not match.",
            )
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Take elements of the input until a predicate is true."
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Take until the element is positive",
                example: "[-1 -2 9 1] | take until {|x| $x > 0 }",
                result: Some(Value::test_list(vec![
                    Value::test_int(-1),
                    Value::test_int(-2),
                ])),
            },
            Example {
                description: "Take until the element is positive using stored condition",
                example: "let cond = {|x| $x > 0 }; [-1 -2 9 1] | take until $cond",
                result: Some(Value::test_list(vec![
                    Value::test_int(-1),
                    Value::test_int(-2),
                ])),
            },
            Example {
                description: "Take until the field value is positive",
                example: "[{a: -1} {a: -2} {a: 9} {a: 1}] | take until {|x| $x.a > 0 }",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "a" => Value::test_int(-1),
                    }),
                    Value::test_record(record! {
                        "a" => Value::test_int(-2),
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
        let head = call.head;
        let closure: Closure = call.req(engine_state, stack, 0)?;

        let mut closure = ClosureEval::new(engine_state, stack, closure);

        let metadata = input.metadata();
        Ok(input
            .into_iter_strict(head)?
            .take_while(move |value| {
                closure
                    .run_with_value(value.clone())
                    .map(|data| data.into_value(head).is_false())
                    .unwrap_or(false)
            })
            .into_pipeline_data_with_metadata(metadata, engine_state.ctrlc.clone()))
    }
}

#[cfg(test)]
mod tests {
    use crate::TakeUntil;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(TakeUntil)
    }
}
