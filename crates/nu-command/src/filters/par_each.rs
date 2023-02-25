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
                (Type::Table(vec![]), Type::List(Box::new(Type::Any))),
            ])
            .required(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any, SyntaxShape::Int])),
                "the closure to run",
            )
            .category(Category::Filters)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "[1 2 3] | par-each { 2 * $in }",
                description:
                    "Multiplies each number. Note that the list will become arbitrarily disordered.",
                result: None,
            },
            Example {
                example: r#"[foo bar baz] | par-each {|e| $e + '!' } | sort"#,
                description: "Output can still be sorted afterward",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_string("bar!"),
                        Value::test_string("baz!"),
                        Value::test_string("foo!"),
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                example: r#"1..3 | enumerate | par-each {|p| update item ($p.item * 2)} | sort-by item | get item"#,
                description: "Enumerate and sort-by can be used to reconstruct the original order",
                result: Some(Value::List {
                    vals: vec![Value::test_int(2), Value::test_int(4), Value::test_int(6)],
                    span: Span::test_data(),
                }),
            },
            Example {
                example: r#"[1 2 3] | enumerate | par-each { |e| if $e.item == 2 { $"found 2 at ($e.index)!"} }"#,
                description:
                    "Iterate over each element, producing a list showing indexes of any 2s",
                result: Some(Value::List {
                    vals: vec![Value::test_string("found 2 at 1!")],
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
        let capture_block: Closure = call.req(engine_state, stack, 0)?;

        let metadata = input.metadata();
        let ctrlc = engine_state.ctrlc.clone();
        let block_id = capture_block.block_id;
        let mut stack = stack.captures_to_stack(&capture_block.captures);
        let redirect_stdout = call.redirect_stdout;
        let redirect_stderr = call.redirect_stderr;

        match input {
            PipelineData::Empty => Ok(PipelineData::Empty),
            PipelineData::Value(Value::Range { val, .. }, ..) => Ok(val
                .into_range_iter(ctrlc.clone())?
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
                    match eval_block_with_early_return(
                        engine_state,
                        &mut stack,
                        block,
                        x.into_pipeline_data(),
                        redirect_stdout,
                        redirect_stderr,
                    ) {
                        Ok(v) => v,
                        Err(error) => Value::Error {
                            error: chain_error_with_input(error, val_span),
                        }
                        .into_pipeline_data(),
                    }
                })
                .collect::<Vec<_>>()
                .into_iter()
                .flatten()
                .into_pipeline_data(ctrlc)),
            PipelineData::Value(Value::List { vals: val, .. }, ..) => Ok(val
                .into_iter()
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
                    match eval_block_with_early_return(
                        engine_state,
                        &mut stack,
                        block,
                        x.into_pipeline_data(),
                        redirect_stdout,
                        redirect_stderr,
                    ) {
                        Ok(v) => v,
                        Err(error) => Value::Error {
                            error: chain_error_with_input(error, val_span),
                        }
                        .into_pipeline_data(),
                    }
                })
                .collect::<Vec<_>>()
                .into_iter()
                .flatten()
                .into_pipeline_data(ctrlc)),
            PipelineData::ListStream(stream, ..) => Ok(stream
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
                    match eval_block_with_early_return(
                        engine_state,
                        &mut stack,
                        block,
                        x.into_pipeline_data(),
                        redirect_stdout,
                        redirect_stderr,
                    ) {
                        Ok(v) => v,
                        Err(error) => Value::Error {
                            error: chain_error_with_input(error, val_span),
                        }
                        .into_pipeline_data(),
                    }
                })
                .collect::<Vec<_>>()
                .into_iter()
                .flatten()
                .into_pipeline_data(ctrlc)),
            PipelineData::ExternalStream { stdout: None, .. } => Ok(PipelineData::empty()),
            PipelineData::ExternalStream {
                stdout: Some(stream),
                ..
            } => Ok(stream
                .par_bridge()
                .map(move |x| {
                    let x = match x {
                        Ok(x) => x,
                        Err(err) => return Value::Error { error: err }.into_pipeline_data(),
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
                        Ok(v) => v,
                        Err(error) => Value::Error { error }.into_pipeline_data(),
                    }
                })
                .collect::<Vec<_>>()
                .into_iter()
                .flatten()
                .into_pipeline_data(ctrlc)),
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
