use nu_engine::{ClosureEval, command_prelude::*};
use nu_protocol::engine::Closure;

#[derive(Clone)]
pub struct TakeWhile;

impl Command for TakeWhile {
    fn name(&self) -> &str {
        "take while"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![
                (Type::table(), Type::table()),
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Any)),
                ),
            ])
            .required(
                "predicate",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                "The predicate that element(s) must match.",
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Take elements of the input while a predicate is true."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Take while the element is negative",
                example: "[-1 -2 9 1] | take while {|x| $x < 0 }",
                result: Some(Value::test_list(vec![
                    Value::test_int(-1),
                    Value::test_int(-2),
                ])),
            },
            Example {
                description: "Take while the element is negative using stored condition",
                example: "let cond = {|x| $x < 0 }; [-1 -2 9 1] | take while $cond",
                result: Some(Value::test_list(vec![
                    Value::test_int(-1),
                    Value::test_int(-2),
                ])),
            },
            Example {
                description: "Take while the field value is negative",
                example: "[{a: -1} {a: -2} {a: 9} {a: 1}] | take while {|x| $x.a < 0 }",
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
                    .and_then(|data| data.into_value(head))
                    .map(|cond| cond.is_true())
                    .unwrap_or(false)
            })
            .into_pipeline_data_with_metadata(head, engine_state.signals().clone(), metadata))
    }
}

#[cfg(test)]
mod tests {
    use crate::TakeWhile;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(TakeWhile)
    }
}
