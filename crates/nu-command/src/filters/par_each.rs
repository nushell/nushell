use nu_engine::{eval_block_with_early_return, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Closure, Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData, Signature,
    Span, SyntaxShape, Type, Value,
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
            .input_output_types(vec![(
                Type::List(Box::new(Type::Any)),
                Type::List(Box::new(Type::Any)),
            )])
            .required(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any, SyntaxShape::Int])),
                "the closure to run",
            )
            .switch(
                "numbered",
                "iterate with an index (deprecated; use a two-parameter closure instead)",
                Some('n'),
            )
            .category(Category::Filters)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "[1 2 3 4 5 6 7 8 9] | par-each {|el| 2 * $el }",
                description:
                    "Multiplies each number.",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_int(2), Value::test_int(4), Value::test_int(6), Value::test_int(8), Value::test_int(10),
                        Value::test_int(12), Value::test_int(14), Value::test_int(16), Value::test_int(18),
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                example: "[bin etc lib] | par-each { { name: $in, length: (ls $in | length) } }",
                description:
                    "Creates a table (a list of records) showing the number of items in each of the given directories.",
                result: None,
            },
            Example {
                example: r#"[1 2 3] | par-each -n { |it| if $it.item == 2 { echo $"found 2 at ($it.index)!"} }"#,
                description: "Iterate over each element, print the matching value and its index",
                result: Some(Value::List {
                    vals: vec![Value::string("found 2 at 1!", Span::test_data())],
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
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let capture_block: Closure = call.req(engine_state, stack, 0)?;

        let numbered = call.has_flag("numbered");
        let metadata = input.metadata();
        let ctrlc = engine_state.ctrlc.clone();
        let block_id = capture_block.block_id;
        let mut stack = stack.captures_to_stack(&capture_block.captures);
        let span = call.head;
        let redirect_stdout = call.redirect_stdout;
        let redirect_stderr = call.redirect_stderr;

        // Closure-executing function, used for almost every case below.
        let mapper = |(idx, x): (usize, Value)| -> PipelineData {
            let block = engine_state.get_block(block_id);

            let mut stack = stack.clone();

            // First index argument
            if let Some(var) = block.signature.get_positional(0) {
                if let Some(var_id) = &var.var_id {
                    // Legacy -n option handled here
                    if numbered {
                        stack.add_var(
                            *var_id,
                            Value::Record {
                                cols: vec!["index".into(), "item".into()],
                                vals: vec![
                                    Value::Int {
                                        val: idx as i64,
                                        span,
                                    },
                                    x.clone(),
                                ],
                                span,
                            },
                        );
                    } else {
                        stack.add_var(*var_id, x.clone());
                    }
                }
            }
            // Optional second index argument
            if let Some(var) = block.signature.get_positional(1) {
                if let Some(var_id) = &var.var_id {
                    stack.add_var(
                        *var_id,
                        Value::Int {
                            val: idx as i64,
                            span,
                        },
                    );
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
        };

        match input {
            PipelineData::Empty => Ok(PipelineData::Empty),
            PipelineData::Value(Value::Range { val, .. }, ..) => Ok(val
                // To ensure that the ordering is preserved when parallelized,
                // the Range is cast to Vec and then made into_par_iter(),
                // instead of calling par_bridge() on the RangeIterator.
                // More efficient methods of doing this are welcome.
                .into_range_iter(ctrlc.clone())?
                .collect::<Vec<_>>()
                .into_par_iter()
                .enumerate()
                .map(mapper)
                .collect::<Vec<_>>()
                .into_iter()
                .flatten()
                .into_pipeline_data(ctrlc)),
            PipelineData::Value(Value::List { vals: val, .. }, ..) => Ok(val
                .into_par_iter()
                .enumerate()
                .map(mapper)
                .collect::<Vec<_>>()
                .into_iter()
                .flatten()
                .into_pipeline_data(ctrlc)),
            PipelineData::ListStream(stream, ..) => Ok(stream
                // To ensure that the ordering is preserved when parallelized,
                // the ListStream is cast to Vec and then made into_par_iter(),
                // instead of calling par_bridge() on the iterator.
                // More efficient methods of doing this are welcome.
                .into_iter()
                .collect::<Vec<_>>()
                .into_par_iter()
                .enumerate()
                .map(mapper)
                .collect::<Vec<_>>()
                .into_iter()
                .flatten()
                .into_pipeline_data(ctrlc)),
            PipelineData::ExternalStream { stdout: None, .. } => Ok(PipelineData::empty()),
            PipelineData::ExternalStream {
                stdout: Some(stream),
                ..
            } => Ok(stream
                .map(|x| match x {
                    Ok(x) => x,
                    Err(error) => Value::Error { error },
                })
                // To ensure that the ordering is preserved when parallelized,
                // the RawStream is cast to Vec and then made into_par_iter(),
                // instead of calling par_bridge() on the iterator.
                // More efficient methods of doing this are welcome.
                .collect::<Vec<_>>()
                .into_par_iter()
                .enumerate()
                .map(mapper)
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
    use nu_test_support::{nu, pipeline};

    #[test]
    fn uses_optional_index_argument() {
        let actual = nu!(
            cwd: ".", pipeline(
            r#"[7,8,9,10] | par-each {|el ind| $ind } | describe"#
        ));

        assert_eq!(actual.out, "list<int>");
    }

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(ParEach {})
    }
}
