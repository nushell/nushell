use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, Record, ShellError, Signature, Type, Value,
};

use std::thread;

#[derive(Clone)]
pub struct Complete;

impl Command for Complete {
    fn name(&self) -> &str {
        "complete"
    }

    fn signature(&self) -> Signature {
        Signature::build("complete")
            .category(Category::System)
            .input_output_types(vec![(Type::Any, Type::Record(vec![]))])
    }

    fn usage(&self) -> &str {
        "Capture the outputs and exit code from an external piped in command in a nushell table."
    }

    fn extra_usage(&self) -> &str {
        r#"In order to capture stdout, stderr, and exit_code, externally piped in commands need to be wrapped with `do`"#
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        match input {
            PipelineData::ExternalStream {
                stdout,
                stderr,
                exit_code,
                ..
            } => {
                let mut record = Record::new();

                // use a thread to receive stderr message.
                // Or we may get a deadlock if child process sends out too much bytes to stdout.
                //
                // For example: in normal linux system, stdout pipe's limit is 65535 bytes.
                // if child process sends out 65536 bytes, the process will be hanged because no consumer
                // consumes the first 65535 bytes
                // So we need a thread to receive stderr message, then the current thread can continue to consume
                // stdout messages.
                let stderr_handler = stderr.map(|stderr| {
                    let stderr_span = stderr.span;
                    (
                        thread::Builder::new()
                            .name("stderr consumer".to_string())
                            .spawn(move || {
                                let stderr = stderr.into_bytes()?;
                                if let Ok(st) = String::from_utf8(stderr.item.clone()) {
                                    Ok::<_, ShellError>(Value::string(st, stderr.span))
                                } else {
                                    Ok::<_, ShellError>(Value::binary(stderr.item, stderr.span))
                                }
                            })
                            .expect("failed to create thread"),
                        stderr_span,
                    )
                });

                if let Some(stdout) = stdout {
                    let stdout = stdout.into_bytes()?;
                    record.push(
                        "stdout",
                        if let Ok(st) = String::from_utf8(stdout.item.clone()) {
                            Value::string(st, stdout.span)
                        } else {
                            Value::binary(stdout.item, stdout.span)
                        },
                    )
                }

                if let Some((handler, stderr_span)) = stderr_handler {
                    let res = handler.join().map_err(|err| ShellError::ExternalCommand {
                        label: "Fail to receive external commands stderr message".to_string(),
                        help: format!("{err:?}"),
                        span: stderr_span,
                    })??;
                    record.push("stderr", res)
                };

                if let Some(exit_code) = exit_code {
                    let mut v: Vec<_> = exit_code.collect();

                    if let Some(v) = v.pop() {
                        record.push("exit_code", v);
                    }
                }

                Ok(Value::record(record, call.head).into_pipeline_data())
            }
            _ => Err(ShellError::GenericError {
                error: "Complete only works with external streams".into(),
                msg: "complete only works on external streams".into(),
                span: Some(call.head),
                help: None,
                inner: vec![],
            }),
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description:
                    "Run the external command to completion, capturing stdout and exit_code",
                example: "^external arg1 | complete",
                result: None,
            },
            Example {
                description:
                    "Run external command to completion, capturing, stdout, stderr and exit_code",
                example: "do { ^external arg1 } | complete",
                result: None,
            },
        ]
    }
}
