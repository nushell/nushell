use nu_engine::{command_prelude::*, get_eval_block_with_early_return, redirect_env};
#[cfg(feature = "os")]
use nu_protocol::process::{ChildPipe, ChildProcess};
use nu_protocol::{
    ByteStream, ByteStreamSource, OutDest, engine::Closure, shell_error::io::IoError,
};

use std::{
    io::{Cursor, Read},
    thread,
};

#[derive(Clone)]
pub struct Do;

impl Command for Do {
    fn name(&self) -> &str {
        "do"
    }

    fn description(&self) -> &str {
        "Run a closure, providing it with the pipeline input."
    }

    fn signature(&self) -> Signature {
        Signature::build("do")
            .required("closure", SyntaxShape::Closure(None), "The closure to run.")
            .input_output_types(vec![(Type::Any, Type::Any)])
            .switch(
                "ignore-errors",
                "ignore errors as the closure runs",
                Some('i'),
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
        let head = call.head;
        let block: Closure = call.req(engine_state, caller_stack, 0)?;
        let rest: Vec<Value> = call.rest(engine_state, caller_stack, 1)?;
        let ignore_all_errors = call.has_flag(engine_state, caller_stack, "ignore-errors")?;

        let capture_errors = call.has_flag(engine_state, caller_stack, "capture-errors")?;
        let has_env = call.has_flag(engine_state, caller_stack, "env")?;

        let mut callee_stack = caller_stack.captures_to_stack_preserve_out_dest(block.captures);
        let block = engine_state.get_block(block.block_id);

        bind_args_to(&mut callee_stack, &block.signature, rest, head)?;
        let eval_block_with_early_return = get_eval_block_with_early_return(engine_state);

        let result = eval_block_with_early_return(engine_state, &mut callee_stack, block, input);

        if has_env {
            // Merge the block's environment to the current stack
            redirect_env(engine_state, caller_stack, &callee_stack);
        }

        match result {
            Ok(PipelineData::ByteStream(stream, metadata)) if capture_errors => {
                let span = stream.span();
                #[cfg(not(feature = "os"))]
                return Err(ShellError::DisabledOsSupport {
                    msg: "Cannot create a thread to receive stdout message.".to_string(),
                    span,
                });

                #[cfg(feature = "os")]
                match stream.into_child() {
                    Ok(mut child) => {
                        // Use a thread to receive stdout message.
                        // Or we may get a deadlock if child process sends out too much bytes to stderr.
                        //
                        // For example: in normal linux system, stderr pipe's limit is 65535 bytes.
                        // if child process sends out 65536 bytes, the process will be hanged because no consumer
                        // consumes the first 65535 bytes
                        // So we need a thread to receive stdout message, then the current thread can continue to consume
                        // stderr messages.
                        let stdout_handler = child
                            .stdout
                            .take()
                            .map(|mut stdout| {
                                thread::Builder::new()
                                    .name("stdout consumer".to_string())
                                    .spawn(move || {
                                        let mut buf = Vec::new();
                                        stdout.read_to_end(&mut buf).map_err(|err| {
                                            IoError::new_internal(
                                                err,
                                                "Could not read stdout to end",
                                                nu_protocol::location!(),
                                            )
                                        })?;
                                        Ok::<_, ShellError>(buf)
                                    })
                                    .map_err(|err| IoError::new(err, head, None))
                            })
                            .transpose()?;

                        // Intercept stderr so we can return it in the error if the exit code is non-zero.
                        // The threading issues mentioned above dictate why we also need to intercept stdout.
                        let stderr_msg = match child.stderr.take() {
                            None => String::new(),
                            Some(mut stderr) => {
                                let mut buf = String::new();
                                stderr
                                    .read_to_string(&mut buf)
                                    .map_err(|err| IoError::new(err, span, None))?;
                                buf
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
                                Ok(res) => Some(res?),
                            }
                        } else {
                            None
                        };

                        child.ignore_error(false);
                        child.wait()?;

                        let mut child = ChildProcess::from_raw(None, None, None, span);
                        if let Some(stdout) = stdout {
                            child.stdout = Some(ChildPipe::Tee(Box::new(Cursor::new(stdout))));
                        }
                        if !stderr_msg.is_empty() {
                            child.stderr = Some(ChildPipe::Tee(Box::new(Cursor::new(stderr_msg))));
                        }
                        Ok(PipelineData::byte_stream(
                            ByteStream::child(child, span),
                            metadata,
                        ))
                    }
                    Err(stream) => Ok(PipelineData::byte_stream(stream, metadata)),
                }
            }
            Ok(PipelineData::ByteStream(mut stream, metadata))
                if ignore_all_errors
                    && !matches!(
                        caller_stack.stdout(),
                        OutDest::Pipe | OutDest::PipeSeparate | OutDest::Value
                    ) =>
            {
                #[cfg(feature = "os")]
                if let ByteStreamSource::Child(child) = stream.source_mut() {
                    child.ignore_error(true);
                }
                Ok(PipelineData::byte_stream(stream, metadata))
            }
            Ok(PipelineData::Value(Value::Error { .. }, ..)) | Err(_) if ignore_all_errors => {
                Ok(PipelineData::empty())
            }
            Ok(PipelineData::ListStream(stream, metadata)) if ignore_all_errors => {
                let stream = stream.map(move |value| {
                    if let Value::Error { .. } = value {
                        Value::nothing(head)
                    } else {
                        value
                    }
                });
                Ok(PipelineData::list_stream(stream, metadata))
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
                description: "Abort the pipeline if a program returns a non-zero exit code",
                example: r#"do --capture-errors { nu --commands 'exit 1' } | myscarycommand"#,
                result: None,
            },
            Example {
                description: "Run the closure with a positional, type-checked parameter",
                example: r#"do {|x:int| 100 + $x } 77"#,
                result: Some(Value::test_int(177)),
            },
            Example {
                description: "Run the closure with pipeline input",
                example: r#"77 | do { 100 + $in }"#,
                result: Some(Value::test_int(177)),
            },
            Example {
                description: "Run the closure with a default parameter value",
                example: r#"77 | do {|x=100| $x + $in }"#,
                result: Some(Value::test_int(177)),
            },
            Example {
                description: "Run the closure with two positional parameters",
                example: r#"do {|x,y| $x + $y } 77 100"#,
                result: Some(Value::test_int(177)),
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
            if !result.is_subtype_of(&param_type) {
                return Err(ShellError::CantConvert {
                    to_type: param.shape.to_type().to_string(),
                    from_type: result.get_type().to_string(),
                    span: result.span(),
                    help: None,
                });
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

        for result in val_iter {
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
