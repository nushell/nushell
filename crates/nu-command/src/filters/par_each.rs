use nu_engine::{eval_block_with_early_return, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Closure, Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData, ShellError,
    Signature, Span, SpannedValue, SyntaxShape, Type,
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
            .required(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any, SyntaxShape::Int])),
                "the closure to run",
            )
            .allow_variants_without_examples(true)
            .category(Category::Filters)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "[1 2 3] | par-each {|| 2 * $in }",
                description:
                    "Multiplies each number. Note that the list will become arbitrarily disordered.",
                result: None,
            },
            Example {
                example: r#"[foo bar baz] | par-each {|e| $e + '!' } | sort"#,
                description: "Output can still be sorted afterward",
                result: Some(SpannedValue::List {
                    vals: vec![
                        SpannedValue::test_string("bar!"),
                        SpannedValue::test_string("baz!"),
                        SpannedValue::test_string("foo!"),
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                example: r#"1..3 | enumerate | par-each {|p| update item ($p.item * 2)} | sort-by item | get item"#,
                description: "Enumerate and sort-by can be used to reconstruct the original order",
                result: Some(SpannedValue::List {
                    vals: vec![
                        SpannedValue::test_int(2),
                        SpannedValue::test_int(4),
                        SpannedValue::test_int(6),
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                example: r#"[1 2 3] | enumerate | par-each { |e| if $e.item == 2 { $"found 2 at ($e.index)!"} }"#,
                description:
                    "Iterate over each element, producing a list showing indexes of any 2s",
                result: Some(SpannedValue::List {
                    vals: vec![SpannedValue::test_string("found 2 at 1!")],
                    span: Span::test_data(),
                }),
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
                Err(e) => Err(e).map_err(|e| {
                    ShellError::GenericError(
                        "Error creating thread pool".into(),
                        e.to_string(),
                        Some(Span::unknown()),
                        None,
                        Vec::new(),
                    )
                }),
                Ok(pool) => Ok(pool),
            }
        }

        let capture_block: Closure = call.req(engine_state, stack, 0)?;
        let threads: Option<usize> = call.get_flag(engine_state, stack, "threads")?;
        let max_threads = threads.unwrap_or(0);
        let metadata = input.metadata();
        let ctrlc = engine_state.ctrlc.clone();
        let outer_ctrlc = engine_state.ctrlc.clone();
        let block_id = capture_block.block_id;
        let mut stack = stack.captures_to_stack(&capture_block.captures);
        let span = call.head;
        let redirect_stdout = call.redirect_stdout;
        let redirect_stderr = call.redirect_stderr;

        match input {
            PipelineData::Empty => Ok(PipelineData::Empty),
            PipelineData::Value(SpannedValue::Range { val, .. }, ..) => {
                Ok(create_pool(max_threads)?.install(|| {
                    val.into_range_iter(ctrlc.clone())
                        .expect("unable to create a range iterator")
                        .par_bridge()
                        .map(move |x| {
                            let block = engine_state.get_block(block_id);

                            let mut stack = stack.clone();

                            if let Some(var) = block.signature.get_positional(0) {
                                if let Some(var_id) = &var.var_id {
                                    stack.add_var(*var_id, x.clone());
                                }
                            }

                            let val_span = x.span();
                            let x_is_error = x.is_error();

                            match eval_block_with_early_return(
                                engine_state,
                                &mut stack,
                                block,
                                x.into_pipeline_data(),
                                redirect_stdout,
                                redirect_stderr,
                            ) {
                                Ok(v) => v.into_value(span),

                                Err(error) => SpannedValue::Error {
                                    error: Box::new(chain_error_with_input(
                                        error, x_is_error, val_span,
                                    )),
                                    span: val_span,
                                },
                            }
                        })
                        .collect::<Vec<_>>()
                        .into_iter()
                        .into_pipeline_data(ctrlc)
                }))
            }
            PipelineData::Value(SpannedValue::List { vals: val, .. }, ..) => {
                Ok(create_pool(max_threads)?.install(|| {
                    val.par_iter()
                        .map(move |x| {
                            let block = engine_state.get_block(block_id);

                            let mut stack = stack.clone();

                            if let Some(var) = block.signature.get_positional(0) {
                                if let Some(var_id) = &var.var_id {
                                    stack.add_var(*var_id, x.clone());
                                }
                            }

                            let val_span = x.span();
                            let x_is_error = x.is_error();

                            match eval_block_with_early_return(
                                engine_state,
                                &mut stack,
                                block,
                                x.clone().into_pipeline_data(),
                                redirect_stdout,
                                redirect_stderr,
                            ) {
                                Ok(v) => v.into_value(span),
                                Err(error) => SpannedValue::Error {
                                    error: Box::new(chain_error_with_input(
                                        error, x_is_error, val_span,
                                    )),
                                    span: val_span,
                                },
                            }
                        })
                        .collect::<Vec<_>>()
                        .into_iter()
                        .into_pipeline_data(ctrlc)
                }))
            }
            PipelineData::ListStream(stream, ..) => Ok(create_pool(max_threads)?.install(|| {
                stream
                    .par_bridge()
                    .map(move |x| {
                        let block = engine_state.get_block(block_id);

                        let mut stack = stack.clone();

                        if let Some(var) = block.signature.get_positional(0) {
                            if let Some(var_id) = &var.var_id {
                                stack.add_var(*var_id, x.clone());
                            }
                        }

                        let val_span = x.span();
                        let x_is_error = x.is_error();

                        match eval_block_with_early_return(
                            engine_state,
                            &mut stack,
                            block,
                            x.into_pipeline_data(),
                            redirect_stdout,
                            redirect_stderr,
                        ) {
                            Ok(v) => v.into_value(span),
                            Err(error) => SpannedValue::Error {
                                error: Box::new(chain_error_with_input(
                                    error, x_is_error, val_span,
                                )),
                                span: val_span,
                            },
                        }
                    })
                    .collect::<Vec<_>>()
                    .into_iter()
                    .into_pipeline_data(ctrlc)
            })),
            PipelineData::ExternalStream { stdout: None, .. } => Ok(PipelineData::empty()),
            PipelineData::ExternalStream {
                stdout: Some(stream),
                ..
            } => Ok(create_pool(max_threads)?.install(|| {
                stream
                    .par_bridge()
                    .map(move |x| {
                        let x = match x {
                            Ok(x) => x,
                            Err(err) => {
                                return SpannedValue::Error {
                                    error: Box::new(err),
                                    span,
                                }
                            }
                        };

                        let block = engine_state.get_block(block_id);

                        let mut stack = stack.clone();

                        if let Some(var) = block.signature.get_positional(0) {
                            if let Some(var_id) = &var.var_id {
                                stack.add_var(*var_id, x.clone());
                            }
                        }

                        match eval_block_with_early_return(
                            engine_state,
                            &mut stack,
                            block,
                            x.into_pipeline_data(),
                            redirect_stdout,
                            redirect_stderr,
                        ) {
                            Ok(v) => v.into_value(span),
                            Err(error) => SpannedValue::Error {
                                error: Box::new(error),
                                span,
                            },
                        }
                    })
                    .collect::<Vec<_>>()
                    .into_iter()
                    .into_pipeline_data(ctrlc)
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
