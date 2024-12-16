use super::utils::chain_error_with_input;
use nu_engine::{command_prelude::*, ClosureEval, ClosureEvalOnce};
use nu_protocol::engine::Closure;

#[derive(Clone)]
pub struct Filter;

impl Command for Filter {
    fn name(&self) -> &str {
        "filter"
    }

    fn description(&self) -> &str {
        "Filter values based on a predicate closure."
    }

    fn extra_description(&self) -> &str {
        r#"This command works similar to 'where' but allows reading the predicate closure from
a variable. On the other hand, the "row condition" syntax is not supported."#
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("filter")
            .input_output_types(vec![
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Any)),
                ),
                (Type::Range, Type::List(Box::new(Type::Any))),
            ])
            .required(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                "Predicate closure.",
            )
            .category(Category::Filters)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["where", "find", "search", "condition"]
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
                    .filter_map(move |value| {
                        match closure
                            .run_with_value(value.clone())
                            .and_then(|data| data.into_value(head))
                        {
                            Ok(cond) => cond.is_true().then_some(value),
                            Err(err) => {
                                let span = value.span();
                                let err = chain_error_with_input(err, value.is_error(), span);
                                Some(Value::error(err, span))
                            }
                        }
                    })
                    .into_pipeline_data(head, engine_state.signals().clone()))
            }
            PipelineData::ByteStream(stream, ..) => {
                if let Some(chunks) = stream.chunks() {
                    let mut closure = ClosureEval::new(engine_state, stack, closure);
                    Ok(chunks
                        .into_iter()
                        .filter_map(move |value| {
                            let value = match value {
                                Ok(value) => value,
                                Err(err) => return Some(Value::error(err, head)),
                            };

                            match closure
                                .run_with_value(value.clone())
                                .and_then(|data| data.into_value(head))
                            {
                                Ok(cond) => cond.is_true().then_some(value),
                                Err(err) => {
                                    let span = value.span();
                                    let err = chain_error_with_input(err, value.is_error(), span);
                                    Some(Value::error(err, span))
                                }
                            }
                        })
                        .into_pipeline_data(head, engine_state.signals().clone()))
                } else {
                    Ok(PipelineData::Empty)
                }
            }
            // This match allows non-iterables to be accepted,
            // which is currently considered undesirable (Nov 2022).
            PipelineData::Value(value, ..) => {
                let result = ClosureEvalOnce::new(engine_state, stack, closure)
                    .run_with_value(value.clone())
                    .and_then(|data| data.into_value(head));

                Ok(match result {
                    Ok(cond) => cond.is_true().then_some(value),
                    Err(err) => {
                        let span = value.span();
                        let err = chain_error_with_input(err, value.is_error(), span);
                        Some(Value::error(err, span))
                    }
                }
                .into_pipeline_data(head, engine_state.signals().clone()))
            }
        }
        .map(|data| data.set_metadata(metadata))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Filter items of a list according to a condition",
                example: "[1 2] | filter {|x| $x > 1}",
                result: Some(Value::test_list(vec![Value::test_int(2)])),
            },
            Example {
                description: "Filter rows of a table according to a condition",
                example: "[{a: 1} {a: 2}] | filter {|x| $x.a > 1}",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "a" => Value::test_int(2),
                })])),
            },
            Example {
                description: "Filter rows of a table according to a stored condition",
                example: "let cond = {|x| $x.a > 1}; [{a: 1} {a: 2}] | filter $cond",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "a" => Value::test_int(2),
                })])),
            },
            Example {
                description: "Filter items of a range according to a condition",
                example: "9..13 | filter {|el| $el mod 2 != 0}",
                result: Some(Value::test_list(vec![
                    Value::test_int(9),
                    Value::test_int(11),
                    Value::test_int(13),
                ])),
            },
            Example {
                description: "List all numbers above 3, using an existing closure condition",
                example: "let a = {$in > 3}; [1, 2, 5, 6] | filter $a",
                result: None, // TODO: This should work
                              // result: Some(Value::test_list(
                              //     vec![
                              //         Value::Int {
                              //             val: 5,
                              //             Span::test_data(),
                              //         },
                              //         Value::Int {
                              //             val: 6,
                              //             span: Span::test_data(),
                              //         },
                              //     ],
                              // }),
            },
        ]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Filter {})
    }
}
