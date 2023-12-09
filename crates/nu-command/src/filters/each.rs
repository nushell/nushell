use super::utils::chain_error_with_input;
use nu_engine::{eval_block_with_early_return, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Closure, Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData, ShellError,
    Signature, Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct Each;

impl Command for Each {
    fn name(&self) -> &str {
        "each"
    }

    fn usage(&self) -> &str {
        "Run a closure on each row of the input list, creating a new list with the results."
    }

    fn extra_usage(&self) -> &str {
        r#"Since tables are lists of records, passing a table into 'each' will
iterate over each record, not necessarily each cell within it.

Avoid passing single records to this command. Since a record is a
one-row structure, 'each' will only run once, behaving similar to 'do'.
To iterate over a record's values, try converting it to a table
with 'transpose' first."#
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["for", "loop", "iterate", "map"]
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("each")
            .input_output_types(vec![
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Any)),
                ),
                (Type::Table(vec![]), Type::List(Box::new(Type::Any))),
                (Type::Any, Type::Any),
            ])
            .required(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any, SyntaxShape::Int])),
                "the closure to run",
            )
            .switch("keep-empty", "keep empty result cells", Some('k'))
            .allow_variants_without_examples(true)
            .category(Category::Filters)
    }

    fn examples(&self) -> Vec<Example> {
        let stream_test_1 = vec![Value::test_int(2), Value::test_int(4), Value::test_int(6)];

        let stream_test_2 = vec![
            Value::nothing(Span::test_data()),
            Value::test_string("found 2!"),
            Value::nothing(Span::test_data()),
        ];

        vec![
            Example {
                example: "[1 2 3] | each {|e| 2 * $e }",
                description: "Multiplies elements in the list",
                result: Some(Value::list(stream_test_1, Span::test_data())),
            },
            Example {
                example: "{major:2, minor:1, patch:4} | values | each {|| into string }",
                description: "Produce a list of values in the record, converted to string",
                result: Some(Value::list(
                    vec![
                        Value::test_string("2"),
                        Value::test_string("1"),
                        Value::test_string("4"),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                example: r#"[1 2 3 2] | each {|e| if $e == 2 { "two" } }"#,
                description: "Produce a list that has \"two\" for each 2 in the input",
                result: Some(Value::list(
                    vec![Value::test_string("two"), Value::test_string("two")],
                    Span::test_data(),
                )),
            },
            Example {
                example: r#"[1 2 3] | enumerate | each {|e| if $e.item == 2 { $"found 2 at ($e.index)!"} }"#,
                description:
                    "Iterate over each element, producing a list showing indexes of any 2s",
                result: Some(Value::list(
                    vec![Value::test_string("found 2 at 1!")],
                    Span::test_data(),
                )),
            },
            Example {
                example: r#"[1 2 3] | each --keep-empty {|e| if $e == 2 { "found 2!"} }"#,
                description: "Iterate over each element, keeping null results",
                result: Some(Value::list(stream_test_2, Span::test_data())),
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

        let keep_empty = call.has_flag("keep-empty");

        let metadata = input.metadata();
        let ctrlc = engine_state.ctrlc.clone();
        let outer_ctrlc = engine_state.ctrlc.clone();
        let engine_state = engine_state.clone();
        let block = engine_state.get_block(capture_block.block_id).clone();
        let mut stack = stack.captures_to_stack(capture_block.captures);
        let orig_env_vars = stack.env_vars.clone();
        let orig_env_hidden = stack.env_hidden.clone();
        let span = call.head;
        let redirect_stdout = call.redirect_stdout;
        let redirect_stderr = call.redirect_stderr;

        match input {
            PipelineData::Empty => Ok(PipelineData::Empty),
            PipelineData::Value(Value::Range { .. }, ..)
            | PipelineData::Value(Value::List { .. }, ..)
            | PipelineData::ListStream { .. } => Ok(input
                .into_iter()
                .map_while(move |x| {
                    // with_env() is used here to ensure that each iteration uses
                    // a different set of environment variables.
                    // Hence, a 'cd' in the first loop won't affect the next loop.
                    stack.with_env(&orig_env_vars, &orig_env_hidden);

                    if let Some(var) = block.signature.get_positional(0) {
                        if let Some(var_id) = &var.var_id {
                            stack.add_var(*var_id, x.clone());
                        }
                    }

                    let input_span = x.span();
                    let x_is_error = x.is_error();
                    match eval_block_with_early_return(
                        &engine_state,
                        &mut stack,
                        &block,
                        x.into_pipeline_data(),
                        redirect_stdout,
                        redirect_stderr,
                    ) {
                        Ok(v) => Some(v.into_value(span)),
                        Err(ShellError::Continue { span }) => Some(Value::nothing(span)),
                        Err(ShellError::Break { .. }) => None,
                        Err(error) => {
                            let error = chain_error_with_input(error, x_is_error, input_span);
                            Some(Value::error(error, input_span))
                        }
                    }
                })
                .into_pipeline_data(ctrlc)),
            PipelineData::ExternalStream { stdout: None, .. } => Ok(PipelineData::empty()),
            PipelineData::ExternalStream {
                stdout: Some(stream),
                ..
            } => Ok(stream
                .into_iter()
                .map_while(move |x| {
                    // with_env() is used here to ensure that each iteration uses
                    // a different set of environment variables.
                    // Hence, a 'cd' in the first loop won't affect the next loop.
                    stack.with_env(&orig_env_vars, &orig_env_hidden);

                    let x = match x {
                        Ok(x) => x,
                        Err(ShellError::Continue { span }) => return Some(Value::nothing(span)),
                        Err(ShellError::Break { .. }) => return None,
                        Err(err) => return Some(Value::error(err, span)),
                    };

                    if let Some(var) = block.signature.get_positional(0) {
                        if let Some(var_id) = &var.var_id {
                            stack.add_var(*var_id, x.clone());
                        }
                    }

                    let input_span = x.span();
                    let x_is_error = x.is_error();

                    match eval_block_with_early_return(
                        &engine_state,
                        &mut stack,
                        &block,
                        x.into_pipeline_data(),
                        redirect_stdout,
                        redirect_stderr,
                    ) {
                        Ok(v) => Some(v.into_value(span)),
                        Err(ShellError::Continue { span }) => Some(Value::nothing(span)),
                        Err(ShellError::Break { .. }) => None,
                        Err(error) => {
                            let error = chain_error_with_input(error, x_is_error, input_span);
                            Some(Value::error(error, input_span))
                        }
                    }
                })
                .into_pipeline_data(ctrlc)),
            // This match allows non-iterables to be accepted,
            // which is currently considered undesirable (Nov 2022).
            PipelineData::Value(x, ..) => {
                if let Some(var) = block.signature.get_positional(0) {
                    if let Some(var_id) = &var.var_id {
                        stack.add_var(*var_id, x.clone());
                    }
                }

                eval_block_with_early_return(
                    &engine_state,
                    &mut stack,
                    &block,
                    x.into_pipeline_data(),
                    redirect_stdout,
                    redirect_stderr,
                )
            }
        }
        .and_then(|x| {
            x.filter(
                move |x| if !keep_empty { !x.is_nothing() } else { true },
                outer_ctrlc,
            )
        })
        .map(|x| x.set_metadata(metadata))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Each {})
    }
}
