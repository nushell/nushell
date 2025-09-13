use nu_engine::{ClosureEval, command_prelude::*};
use nu_protocol::engine::Closure;

#[derive(Clone)]
pub struct SkipWhile;

impl Command for SkipWhile {
    fn name(&self) -> &str {
        "skip while"
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
                "The predicate that skipped element must match.",
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Skip elements of the input while a predicate is true."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["ignore"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Skip while the element is negative",
                example: "[-2 0 2 -1] | skip while {|x| $x < 0 }",
                result: Some(Value::test_list(vec![
                    Value::test_int(0),
                    Value::test_int(2),
                    Value::test_int(-1),
                ])),
            },
            Example {
                description: "Skip while the element is negative using stored condition",
                example: "let cond = {|x| $x < 0 }; [-2 0 2 -1] | skip while $cond",
                result: Some(Value::test_list(vec![
                    Value::test_int(0),
                    Value::test_int(2),
                    Value::test_int(-1),
                ])),
            },
            Example {
                description: "Skip while the field value is negative",
                example: "[{a: -2} {a: 0} {a: 2} {a: -1}] | skip while {|x| $x.a < 0 }",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "a" => Value::test_int(0),
                    }),
                    Value::test_record(record! {
                        "a" => Value::test_int(2),
                    }),
                    Value::test_record(record! {
                        "a" => Value::test_int(-1),
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
            .skip_while(move |value| {
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
    use crate::SkipWhile;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SkipWhile)
    }
}
