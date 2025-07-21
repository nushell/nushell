use super::utils::chain_error_with_input;
use nu_engine::{ClosureEvalOnce, command_prelude::*};
use nu_protocol::{Signals, engine::Closure};
use rayon::prelude::*;

#[derive(Clone)]
pub struct ParEach;

impl Command for ParEach {
    fn name(&self) -> &str {
        "par-each"
    }

    fn description(&self) -> &str {
        "Run a closure on each row of the input list in parallel, creating a new list with the results."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("par-each")
            .input_output_types(vec![
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Any)),
                ),
                (Type::table(), Type::List(Box::new(Type::Any))),
                (Type::Any, Type::Any),
            ])
            .named(
                "threads",
                SyntaxShape::Int,
                "the number of threads to use",
                Some('t'),
            )
            .switch(
                "keep-order",
                "keep sequence of output same as the order of input",
                Some('k'),
            )
            .required(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                "The closure to run.",
            )
            .allow_variants_without_examples(true)
            .category(Category::Filters)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "[1 2 3] | par-each {|e| $e * 2 }",
                description: "Multiplies each number. Note that the list will become arbitrarily disordered.",
                result: None,
            },
            Example {
                example: r#"[1 2 3] | par-each --keep-order {|e| $e * 2 }"#,
                description: "Multiplies each number, keeping an original order",
                result: Some(Value::test_list(vec![
                    Value::test_int(2),
                    Value::test_int(4),
                    Value::test_int(6),
                ])),
            },
            Example {
                example: r#"1..3 | enumerate | par-each {|p| update item ($p.item * 2)} | sort-by item | get item"#,
                description: "Enumerate and sort-by can be used to reconstruct the original order",
                result: Some(Value::test_list(vec![
                    Value::test_int(2),
                    Value::test_int(4),
                    Value::test_int(6),
                ])),
            },
            Example {
                example: r#"[foo bar baz] | par-each {|e| $e + '!' } | sort"#,
                description: "Output can still be sorted afterward",
                result: Some(Value::test_list(vec![
                    Value::test_string("bar!"),
                    Value::test_string("baz!"),
                    Value::test_string("foo!"),
                ])),
            },
            Example {
                example: r#"[1 2 3] | enumerate | par-each { |e| if $e.item == 2 { $"found 2 at ($e.index)!"} }"#,
                description: "Iterate over each element, producing a list showing indexes of any 2s",
                result: Some(Value::test_list(vec![Value::test_string("found 2 at 1!")])),
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
        fn create_pool(num_threads: usize) -> Result<rayon::ThreadPool, ShellError> {
            match rayon::ThreadPoolBuilder::new()
                .num_threads(num_threads)
                .build()
            {
                Err(e) => Err(e).map_err(|e| ShellError::GenericError {
                    error: "Error creating thread pool".into(),
                    msg: e.to_string(),
                    span: Some(Span::unknown()),
                    help: None,
                    inner: vec![],
                }),
                Ok(pool) => Ok(pool),
            }
        }

        let head = call.head;
        let closure: Closure = call.req(engine_state, stack, 0)?;
        let threads: Option<usize> = call.get_flag(engine_state, stack, "threads")?;
        let max_threads = threads.unwrap_or(0);
        let keep_order = call.has_flag(engine_state, stack, "keep-order")?;

        let metadata = input.metadata();

        // A helper function sorts the output if needed
        let apply_order = |mut vec: Vec<(usize, Value)>| {
            if keep_order {
                // It runs inside the rayon's thread pool so parallel sorting can be used.
                // There are no identical indexes, so unstable sorting can be used.
                vec.par_sort_unstable_by_key(|(index, _)| *index);
            }

            vec.into_iter().map(|(_, val)| val)
        };

        match input {
            PipelineData::Empty => Ok(PipelineData::empty()),
            PipelineData::Value(value, ..) => {
                let span = value.span();
                match value {
                    Value::List { vals, .. } => Ok(create_pool(max_threads)?.install(|| {
                        let vec = vals
                            .into_par_iter()
                            .enumerate()
                            .map(move |(index, value)| {
                                let span = value.span();
                                let is_error = value.is_error();
                                let value =
                                    ClosureEvalOnce::new(engine_state, stack, closure.clone())
                                        .run_with_value(value)
                                        .and_then(|data| data.into_value(span))
                                        .unwrap_or_else(|err| {
                                            Value::error(
                                                chain_error_with_input(err, is_error, span),
                                                span,
                                            )
                                        });

                                (index, value)
                            })
                            .collect::<Vec<_>>();

                        apply_order(vec).into_pipeline_data(span, engine_state.signals().clone())
                    })),
                    Value::Range { val, .. } => Ok(create_pool(max_threads)?.install(|| {
                        let vec = val
                            .into_range_iter(span, Signals::empty())
                            .enumerate()
                            .par_bridge()
                            .map(move |(index, value)| {
                                let span = value.span();
                                let is_error = value.is_error();
                                let value =
                                    ClosureEvalOnce::new(engine_state, stack, closure.clone())
                                        .run_with_value(value)
                                        .and_then(|data| data.into_value(span))
                                        .unwrap_or_else(|err| {
                                            Value::error(
                                                chain_error_with_input(err, is_error, span),
                                                span,
                                            )
                                        });

                                (index, value)
                            })
                            .collect::<Vec<_>>();

                        apply_order(vec).into_pipeline_data(span, engine_state.signals().clone())
                    })),
                    // This match allows non-iterables to be accepted,
                    // which is currently considered undesirable (Nov 2022).
                    value => {
                        ClosureEvalOnce::new(engine_state, stack, closure).run_with_value(value)
                    }
                }
            }
            PipelineData::ListStream(stream, ..) => Ok(create_pool(max_threads)?.install(|| {
                let vec = stream
                    .into_iter()
                    .enumerate()
                    .par_bridge()
                    .map(move |(index, value)| {
                        let span = value.span();
                        let is_error = value.is_error();
                        let value = ClosureEvalOnce::new(engine_state, stack, closure.clone())
                            .run_with_value(value)
                            .and_then(|data| data.into_value(head))
                            .unwrap_or_else(|err| {
                                Value::error(chain_error_with_input(err, is_error, span), span)
                            });

                        (index, value)
                    })
                    .collect::<Vec<_>>();

                apply_order(vec).into_pipeline_data(head, engine_state.signals().clone())
            })),
            PipelineData::ByteStream(stream, ..) => {
                if let Some(chunks) = stream.chunks() {
                    Ok(create_pool(max_threads)?.install(|| {
                        let vec = chunks
                            .enumerate()
                            .par_bridge()
                            .map(move |(index, value)| {
                                let value = match value {
                                    Ok(value) => value,
                                    Err(err) => return (index, Value::error(err, head)),
                                };

                                let value =
                                    ClosureEvalOnce::new(engine_state, stack, closure.clone())
                                        .run_with_value(value)
                                        .and_then(|data| data.into_value(head))
                                        .unwrap_or_else(|err| Value::error(err, head));

                                (index, value)
                            })
                            .collect::<Vec<_>>();

                        apply_order(vec).into_pipeline_data(head, engine_state.signals().clone())
                    }))
                } else {
                    Ok(PipelineData::empty())
                }
            }
        }
        .and_then(|x| x.filter(|v| !v.is_nothing(), engine_state.signals()))
        .map(|data| data.set_metadata(metadata))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(ParEach {})
    }
}
