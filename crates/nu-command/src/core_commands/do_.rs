use nu_engine::{eval_block, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Closure, Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, ListStream, PipelineData, RawStream, ShellError, Signature, SyntaxShape,
    Type, Value,
};

#[derive(Clone)]
pub struct Do;

impl Command for Do {
    fn name(&self) -> &str {
        "do"
    }

    fn usage(&self) -> &str {
        "Run a block"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("do")
            .required("closure", SyntaxShape::Any, "the closure to run")
            .input_output_types(vec![(Type::Any, Type::Any)])
            .switch(
                "ignore-errors",
                "ignore shell errors as the block runs",
                Some('i'),
            )
            .switch(
                "capture-errors",
                "capture errors as the block runs and return it",
                Some('c'),
            )
            .rest("rest", SyntaxShape::Any, "the parameter(s) for the block")
            .category(Category::Core)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let block: Closure = call.req(engine_state, stack, 0)?;
        let rest: Vec<Value> = call.rest(engine_state, stack, 1)?;
        let ignore_errors = call.has_flag("ignore-errors");
        let capture_errors = call.has_flag("capture-errors");

        let mut stack = stack.captures_to_stack(&block.captures);
        let block = engine_state.get_block(block.block_id);

        let params: Vec<_> = block
            .signature
            .required_positional
            .iter()
            .chain(block.signature.optional_positional.iter())
            .collect();

        for param in params.iter().zip(&rest) {
            if let Some(var_id) = param.0.var_id {
                stack.add_var(var_id, param.1.clone())
            }
        }

        if let Some(param) = &block.signature.rest_positional {
            if rest.len() > params.len() {
                let mut rest_items = vec![];

                for r in rest.into_iter().skip(params.len()) {
                    rest_items.push(r);
                }

                let span = if let Some(rest_item) = rest_items.first() {
                    rest_item.span()?
                } else {
                    call.head
                };

                stack.add_var(
                    param
                        .var_id
                        .expect("Internal error: rest positional parameter lacks var_id"),
                    Value::List {
                        vals: rest_items,
                        span,
                    },
                )
            }
        }
        let result = eval_block(
            engine_state,
            &mut stack,
            block,
            input,
            call.redirect_stdout,
            ignore_errors || capture_errors,
        );

        if ignore_errors {
            match result {
                Ok(x) => Ok(x),
                Err(_) => Ok(PipelineData::new(call.head)),
            }
        } else if capture_errors {
            // collect stdout and stderr and check exit code.
            // if exit code is not 0, return back ShellError.
            match result {
                Ok(PipelineData::ExternalStream {
                    stdout,
                    stderr,
                    exit_code,
                    span,
                    metadata,
                }) => {
                    // collect all output first.
                    let mut stderr_ctrlc = None;
                    let stderr_msg = match stderr {
                        None => "".to_string(),
                        Some(stderr_stream) => {
                            stderr_ctrlc = stderr_stream.ctrlc.clone();
                            stderr_stream.into_string().map(|s| s.item)?
                        }
                    };

                    let mut stdout_ctrlc = None;
                    let stdout_msg = match stdout {
                        None => "".to_string(),
                        Some(stdout_stream) => {
                            stdout_ctrlc = stdout_stream.ctrlc.clone();
                            stdout_stream.into_string().map(|s| s.item)?
                        }
                    };

                    let mut exit_code_ctrlc = None;
                    let exit_code: Vec<Value> = match exit_code {
                        None => vec![],
                        Some(exit_code_stream) => {
                            exit_code_ctrlc = exit_code_stream.ctrlc.clone();
                            exit_code_stream.into_iter().collect()
                        }
                    };
                    if let Some(Value::Int { val: code, .. }) = exit_code.last() {
                        // if exit_code is not 0, it indicates error occured, return back Err.
                        if *code != 0 {
                            return Err(ShellError::ExternalCommand(
                                "External command runs to failed".to_string(),
                                stderr_msg,
                                span,
                            ));
                        }
                    }
                    // construct pipeline data to our caller
                    Ok(PipelineData::ExternalStream {
                        stdout: Some(RawStream::new(
                            Box::new(vec![Ok(stdout_msg.into_bytes())].into_iter()),
                            stdout_ctrlc,
                            span,
                        )),
                        stderr: Some(RawStream::new(
                            Box::new(vec![Ok(stderr_msg.into_bytes())].into_iter()),
                            stderr_ctrlc,
                            span,
                        )),
                        exit_code: Some(ListStream::from_stream(
                            exit_code.into_iter(),
                            exit_code_ctrlc,
                        )),
                        span,
                        metadata,
                    })
                }
                Ok(other) => Ok(other),
                Err(e) => Err(e),
            }
        } else {
            result
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Run the block",
                example: r#"do { echo hello }"#,
                result: Some(Value::test_string("hello")),
            },
            Example {
                description: "Run the block and ignore shell errors",
                example: r#"do -i { thisisnotarealcommand }"#,
                result: None,
            },
            Example {
                description: "Abort the pipeline if a program returns a non-zero exit code",
                example: r#"do -c { nu -c 'exit 1' } | myscarycommand"#,
                result: None,
            },
            Example {
                description: "Run the block, with a positional parameter",
                example: r#"do {|x| 100 + $x } 77"#,
                result: Some(Value::test_int(177)),
            },
            Example {
                description: "Run the block, with input",
                example: r#"77 | do {|x| 100 + $in }"#,
                result: None, // TODO: returns 177
            },
        ]
    }
}

mod test {
    #[test]
    fn test_examples() {
        use super::Do;
        use crate::test_examples;
        test_examples(Do {})
    }
}
