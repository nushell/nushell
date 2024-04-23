use nu_engine::{command_prelude::*, get_eval_block_with_early_return, redirect_env};
use nu_protocol::{engine::Closure, ListStream, OutDest, RawStream};
use std::thread;

#[derive(Clone)]
pub struct Do;

impl Command for Do {
    fn name(&self) -> &str {
        "do"
    }

    fn usage(&self) -> &str {
        "Run a closure, providing it with the pipeline input."
    }

    fn signature(&self) -> Signature {
        Signature::build("do")
            .required(
                "closure",
                SyntaxShape::OneOf(vec![SyntaxShape::Closure(None), SyntaxShape::Any]),
                "The closure to run.",
            )
            .input_output_types(vec![(Type::Any, Type::Any)])
            .switch(
                "ignore-errors",
                "ignore errors as the closure runs",
                Some('i'),
            )
            .switch(
                "ignore-shell-errors",
                "ignore shell errors as the closure runs",
                Some('s'),
            )
            .switch(
                "ignore-program-errors",
                "ignore external program errors as the closure runs",
                Some('p'),
            )
            .switch(
                "capture-errors",
                "catch errors as the closure runs, and return them",
                Some('c'),
            )
            .switch(
                "env",
                "keep the environment defined inside the command",
                None,
            )
            .rest(
                "rest",
                SyntaxShape::Any,
                "The parameter(s) for the closure.",
            )
            .category(Category::Core)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        caller_stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let block: Closure = call.req(engine_state, caller_stack, 0)?;
        let rest: Vec<Value> = call.rest(engine_state, caller_stack, 1)?;
        let ignore_all_errors = call.has_flag(engine_state, caller_stack, "ignore-errors")?;
        let ignore_shell_errors = ignore_all_errors
            || call.has_flag(engine_state, caller_stack, "ignore-shell-errors")?;
        let ignore_program_errors = ignore_all_errors
            || call.has_flag(engine_state, caller_stack, "ignore-program-errors")?;
        let capture_errors = call.has_flag(engine_state, caller_stack, "capture-errors")?;
        let has_env = call.has_flag(engine_state, caller_stack, "env")?;

        let mut callee_stack = caller_stack.captures_to_stack_preserve_out_dest(block.captures);
        let block = engine_state.get_block(block.block_id);

        bind_args_to(&mut callee_stack, &block.signature, rest, call.head)?;
        let eval_block_with_early_return = get_eval_block_with_early_return(engine_state);
        let result = eval_block_with_early_return(engine_state, &mut callee_stack, block, input);

        if has_env {
            // Merge the block's environment to the current stack
            redirect_env(engine_state, caller_stack, &callee_stack);
        }

        match result {
            Ok(PipelineData::ExternalStream {
                stdout,
                stderr,
                exit_code,
                span,
                metadata,
                trim_end_newline,
            }) if capture_errors => {
                // Use a thread to receive stdout message.
                // Or we may get a deadlock if child process sends out too much bytes to stderr.
                //
                // For example: in normal linux system, stderr pipe's limit is 65535 bytes.
                // if child process sends out 65536 bytes, the process will be hanged because no consumer
                // consumes the first 65535 bytes
                // So we need a thread to receive stdout message, then the current thread can continue to consume
                // stderr messages.
                let stdout_handler = stdout
                    .map(|stdout_stream| {
                        thread::Builder::new()
                            .name("stderr redirector".to_string())
                            .spawn(move || {
                                let ctrlc = stdout_stream.ctrlc.clone();
                                let span = stdout_stream.span;
                                RawStream::new(
                                    Box::new(std::iter::once(
                                        stdout_stream.into_bytes().map(|s| s.item),
                                    )),
                                    ctrlc,
                                    span,
                                    None,
                                )
                            })
                            .err_span(call.head)
                    })
                    .transpose()?;

                // Intercept stderr so we can return it in the error if the exit code is non-zero.
                // The threading issues mentioned above dictate why we also need to intercept stdout.
                let mut stderr_ctrlc = None;
                let stderr_msg = match stderr {
                    None => "".to_string(),
                    Some(stderr_stream) => {
                        stderr_ctrlc = stderr_stream.ctrlc.clone();
                        stderr_stream.into_string().map(|s| s.item)?
                    }
                };

                let stdout = if let Some(handle) = stdout_handler {
                    match handle.join() {
                        Err(err) => {
                            return Err(ShellError::ExternalCommand {
                                label: "Fail to receive external commands stdout message"
                                    .to_string(),
                                help: format!("{err:?}"),
                                span,
                            });
                        }
                        Ok(res) => Some(res),
                    }
                } else {
                    None
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
                    if *code != 0 {
                        return Err(ShellError::ExternalCommand {
                            label: "External command failed".to_string(),
                            help: stderr_msg,
                            span,
                        });
                    }
                }

                Ok(PipelineData::ExternalStream {
                    stdout,
                    stderr: Some(RawStream::new(
                        Box::new(std::iter::once(Ok(stderr_msg.into_bytes()))),
                        stderr_ctrlc,
                        span,
                        None,
                    )),
                    exit_code: Some(ListStream::from_stream(
                        exit_code.into_iter(),
                        exit_code_ctrlc,
                    )),
                    span,
                    metadata,
                    trim_end_newline,
                })
            }
            Ok(PipelineData::ExternalStream {
                stdout,
                stderr,
                exit_code: _,
                span,
                metadata,
                trim_end_newline,
            }) if ignore_program_errors
                && !matches!(caller_stack.stdout(), OutDest::Pipe | OutDest::Capture) =>
            {
                Ok(PipelineData::ExternalStream {
                    stdout,
                    stderr,
                    exit_code: None,
                    span,
                    metadata,
                    trim_end_newline,
                })
            }
            Ok(PipelineData::Value(Value::Error { .. }, ..)) | Err(_) if ignore_shell_errors => {
                Ok(PipelineData::empty())
            }
            Ok(PipelineData::ListStream(ls, metadata)) if ignore_shell_errors => {
                // check if there is a `Value::Error` in given list stream first.
                let mut values = vec![];
                let ctrlc = ls.ctrlc.clone();
                for v in ls {
                    if let Value::Error { .. } = v {
                        values.push(Value::nothing(call.head));
                    } else {
                        values.push(v)
                    }
                }
                Ok(PipelineData::ListStream(
                    ListStream::from_stream(values.into_iter(), ctrlc),
                    metadata,
                ))
            }
            r => r,
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Run the closure",
                example: r#"do { echo hello }"#,
                result: Some(Value::test_string("hello")),
            },
            Example {
                description: "Run a stored first-class closure",
                example: r#"let text = "I am enclosed"; let hello = {|| echo $text}; do $hello"#,
                result: Some(Value::test_string("I am enclosed")),
            },
            Example {
                description: "Run the closure and ignore both shell and external program errors",
                example: r#"do --ignore-errors { thisisnotarealcommand }"#,
                result: None,
            },
            Example {
                description: "Run the closure and ignore shell errors",
                example: r#"do --ignore-shell-errors { thisisnotarealcommand }"#,
                result: None,
            },
            Example {
                description: "Run the closure and ignore external program errors",
                example: r#"do --ignore-program-errors { nu --commands 'exit 1' }; echo "I'll still run""#,
                result: None,
            },
            Example {
                description: "Abort the pipeline if a program returns a non-zero exit code",
                example: r#"do --capture-errors { nu --commands 'exit 1' } | myscarycommand"#,
                result: None,
            },
            Example {
                description: "Run the closure, with a positional parameter",
                example: r#"do {|x| 100 + $x } 77"#,
                result: Some(Value::test_int(177)),
            },
            Example {
                description: "Run the closure, with input",
                example: r#"77 | do {|x| 100 + $in }"#,
                result: None, // TODO: returns 177
            },
            Example {
                description: "Run the closure and keep changes to the environment",
                example: r#"do --env { $env.foo = 'bar' }; $env.foo"#,
                result: Some(Value::test_string("bar")),
            },
        ]
    }
}

fn bind_args_to(
    stack: &mut Stack,
    signature: &Signature,
    args: Vec<Value>,
    head_span: Span,
) -> Result<(), ShellError> {
    let mut val_iter = args.into_iter();
    for (param, required) in signature
        .required_positional
        .iter()
        .map(|p| (p, true))
        .chain(signature.optional_positional.iter().map(|p| (p, false)))
    {
        let var_id = param
            .var_id
            .expect("internal error: all custom parameters must have var_ids");
        if let Some(result) = val_iter.next() {
            let param_type = param.shape.to_type();
            if required && !result.get_type().is_subtype(&param_type) {
                // need to check if result is an empty list, and param_type is table or list
                // nushell needs to pass type checking for the case.
                let empty_list_matches = result
                    .as_list()
                    .map(|l| l.is_empty() && matches!(param_type, Type::List(_) | Type::Table(_)))
                    .unwrap_or(false);

                if !empty_list_matches {
                    return Err(ShellError::CantConvert {
                        to_type: param.shape.to_type().to_string(),
                        from_type: result.get_type().to_string(),
                        span: result.span(),
                        help: None,
                    });
                }
            }
            stack.add_var(var_id, result);
        } else if let Some(value) = &param.default_value {
            stack.add_var(var_id, value.to_owned())
        } else if !required {
            stack.add_var(var_id, Value::nothing(head_span))
        } else {
            return Err(ShellError::MissingParameter {
                param_name: param.name.to_string(),
                span: head_span,
            });
        }
    }

    if let Some(rest_positional) = &signature.rest_positional {
        let mut rest_items = vec![];

        for result in
            val_iter.skip(signature.required_positional.len() + signature.optional_positional.len())
        {
            rest_items.push(result);
        }

        let span = if let Some(rest_item) = rest_items.first() {
            rest_item.span()
        } else {
            head_span
        };

        stack.add_var(
            rest_positional
                .var_id
                .expect("Internal error: rest positional parameter lacks var_id"),
            Value::list(rest_items, span),
        )
    }
    Ok(())
}

mod test {
    #[test]
    fn test_examples() {
        use super::Do;
        use crate::test_examples;
        test_examples(Do {})
    }
}
