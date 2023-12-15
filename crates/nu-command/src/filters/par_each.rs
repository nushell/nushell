use nu_engine::{eval_block_with_early_return, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Closure, Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData, ShellError,
    Signature, Span, SyntaxShape, Type, Value,
};
use rayon::prelude::*;

use super::utils::chain_error_with_input;

#[derive(Clone)]
pub struct ParEach;

impl Command for ParEach {
    fn name(&self) -> &str {
        "par-each"
    }

    fn usage(&self) -> &str {
        "Run a closure on each row of the input list in parallel, creating a new list with the results."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("par-each")
            .input_output_types(vec![
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Any)),
                ),
                (Type::Range, Type::List(Box::new(Type::Any))),
                (Type::Table(vec![]), Type::List(Box::new(Type::Any))),
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
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any, SyntaxShape::Int])),
                "The closure to run.",
            )
            .allow_variants_without_examples(true)
            .category(Category::Filters)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "[1 2 3] | par-each {|e| $e * 2 }",
                description:
                    "Multiplies each number. Note that the list will become arbitrarily disordered.",
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
                description:
                    "Iterate over each element, producing a list showing indexes of any 2s",
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

        let capture_block: Closure = call.req(engine_state, stack, 0)?;
        let threads: Option<usize> = call.get_flag(engine_state, stack, "threads")?;
        let max_threads = threads.unwrap_or(0);
        let keep_order = call.has_flag("keep-order");
        let metadata = input.metadata();
        let ctrlc = engine_state.ctrlc.clone();
        let outer_ctrlc = engine_state.ctrlc.clone();
        let block_id = capture_block.block_id;
        let mut stack = stack.captures_to_stack(capture_block.captures);
        let span = call.head;
        let redirect_stdout = call.redirect_stdout;
        let redirect_stderr = call.redirect_stderr;

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
            PipelineData::Empty => Ok(PipelineData::Empty),
            PipelineData::Value(Value::Range { val, .. }, ..) => Ok(create_pool(max_threads)?
                .install(|| {
                    let vec = val
                        .into_range_iter(ctrlc.clone())
                        .expect("unable to create a range iterator")
                        .enumerate()
                        .par_bridge()
                        .map(move |(index, x)| {
                            let block = engine_state.get_block(block_id);

                            let mut stack = stack.clone();

                            if let Some(var) = block.signature.get_positional(0) {
                                if let Some(var_id) = &var.var_id {
                                    stack.add_var(*var_id, x.clone());
                                }
                            }

                            let val_span = x.span();
                            let x_is_error = x.is_error();

                            let val = match eval_block_with_early_return(
                                engine_state,
                                &mut stack,
                                block,
                                x.into_pipeline_data(),
                                redirect_stdout,
                                redirect_stderr,
                            ) {
                                Ok(v) => v.into_value(span),
                                Err(error) => Value::error(
                                    chain_error_with_input(error, x_is_error, val_span),
                                    val_span,
                                ),
                            };

                            (index, val)
                        })
                        .collect::<Vec<_>>();

                    apply_order(vec).into_pipeline_data(ctrlc)
                })),
            PipelineData::Value(Value::List { vals: val, .. }, ..) => Ok(create_pool(max_threads)?
                .install(|| {
                    let vec = val
                        .par_iter()
                        .enumerate()
                        .map(move |(index, x)| {
                            let block = engine_state.get_block(block_id);

                            let mut stack = stack.clone();

                            if let Some(var) = block.signature.get_positional(0) {
                                if let Some(var_id) = &var.var_id {
                                    stack.add_var(*var_id, x.clone());
                                }
                            }

                            let val_span = x.span();
                            let x_is_error = x.is_error();

                            let val = match eval_block_with_early_return(
                                engine_state,
                                &mut stack,
                                block,
                                x.clone().into_pipeline_data(),
                                redirect_stdout,
                                redirect_stderr,
                            ) {
                                Ok(v) => v.into_value(span),
                                Err(error) => Value::error(
                                    chain_error_with_input(error, x_is_error, val_span),
                                    val_span,
                                ),
                            };

                            (index, val)
                        })
                        .collect::<Vec<_>>();

                    apply_order(vec).into_pipeline_data(ctrlc)
                })),
            PipelineData::ListStream(stream, ..) => Ok(create_pool(max_threads)?.install(|| {
                let vec = stream
                    .enumerate()
                    .par_bridge()
                    .map(move |(index, x)| {
                        let block = engine_state.get_block(block_id);

                        let mut stack = stack.clone();

                        if let Some(var) = block.signature.get_positional(0) {
                            if let Some(var_id) = &var.var_id {
                                stack.add_var(*var_id, x.clone());
                            }
                        }

                        let val_span = x.span();
                        let x_is_error = x.is_error();

                        let val = match eval_block_with_early_return(
                            engine_state,
                            &mut stack,
                            block,
                            x.into_pipeline_data(),
                            redirect_stdout,
                            redirect_stderr,
                        ) {
                            Ok(v) => v.into_value(span),
                            Err(error) => Value::error(
                                chain_error_with_input(error, x_is_error, val_span),
                                val_span,
                            ),
                        };

                        (index, val)
                    })
                    .collect::<Vec<_>>();

                apply_order(vec).into_pipeline_data(ctrlc)
            })),
            PipelineData::ExternalStream { stdout: None, .. } => Ok(PipelineData::empty()),
            PipelineData::ExternalStream {
                stdout: Some(stream),
                ..
            } => Ok(create_pool(max_threads)?.install(|| {
                let vec = stream
                    .enumerate()
                    .par_bridge()
                    .map(move |(index, x)| {
                        let x = match x {
                            Ok(x) => x,
                            Err(err) => return (index, Value::error(err, span)),
                        };

                        let block = engine_state.get_block(block_id);

                        let mut stack = stack.clone();

                        if let Some(var) = block.signature.get_positional(0) {
                            if let Some(var_id) = &var.var_id {
                                stack.add_var(*var_id, x.clone());
                            }
                        }

                        let val = match eval_block_with_early_return(
                            engine_state,
                            &mut stack,
                            block,
                            x.into_pipeline_data(),
                            redirect_stdout,
                            redirect_stderr,
                        ) {
                            Ok(v) => v.into_value(span),
                            Err(error) => Value::error(error, span),
                        };

                        (index, val)
                    })
                    .collect::<Vec<_>>();

                apply_order(vec).into_pipeline_data(ctrlc)
            })),
            // This match allows non-iterables to be accepted,
            // which is currently considered undesirable (Nov 2022).
            PipelineData::Value(x, ..) => {
                let block = engine_state.get_block(block_id);

                if let Some(var) = block.signature.get_positional(0) {
                    if let Some(var_id) = &var.var_id {
                        stack.add_var(*var_id, x.clone());
                    }
                }

                eval_block_with_early_return(
                    engine_state,
                    &mut stack,
                    block,
                    x.into_pipeline_data(),
                    redirect_stdout,
                    redirect_stderr,
                )
            }
        }
        .and_then(|x| x.filter(|v| !v.is_nothing(), outer_ctrlc))
        .map(|res| res.set_metadata(metadata))
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
