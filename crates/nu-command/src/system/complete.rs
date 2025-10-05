use nu_engine::command_prelude::*;
use nu_protocol::OutDest;

#[derive(Clone)]
pub struct Complete;

impl Command for Complete {
    fn name(&self) -> &str {
        "complete"
    }

    fn signature(&self) -> Signature {
        Signature::build("complete")
            .category(Category::System)
            .input_output_types(vec![(Type::Any, Type::record())])
    }

    fn description(&self) -> &str {
        "Capture the outputs and exit code from an external piped in command in a nushell table."
    }

    fn extra_description(&self) -> &str {
        r#"In order to capture stdout, stderr, and exit_code, externally piped in commands need to be wrapped with `do`"#
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        match input {
            PipelineData::ByteStream(stream, ..) => {
                let Ok(child) = stream.into_child() else {
                    return Err(ShellError::GenericError {
                        error: "Complete only works with external commands".into(),
                        msg: "complete only works on external commands".into(),
                        span: Some(call.head),
                        help: None,
                        inner: vec![],
                    });
                };

                let output = child.wait_with_output()?;
                let exit_code = output.exit_status.code();
                let mut record = Record::new();

                if let Some(stdout) = output.stdout {
                    record.push(
                        "stdout",
                        match String::from_utf8(stdout) {
                            Ok(str) => Value::string(str, head),
                            Err(err) => Value::binary(err.into_bytes(), head),
                        },
                    );
                }

                if let Some(stderr) = output.stderr {
                    record.push(
                        "stderr",
                        match String::from_utf8(stderr) {
                            Ok(str) => Value::string(str, head),
                            Err(err) => Value::binary(err.into_bytes(), head),
                        },
                    );
                }

                record.push("exit_code", Value::int(exit_code.into(), head));

                Ok(Value::record(record, call.head).into_pipeline_data())
            }
            // bubble up errors from the previous command
            PipelineData::Value(Value::Error { error, .. }, _) => Err(*error),
            _ => Err(ShellError::GenericError {
                error: "Complete only works with external commands".into(),
                msg: "complete only works on external commands".into(),
                span: Some(head),
                help: None,
                inner: vec![],
            }),
        }
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Run the external command to completion, capturing stdout, stderr, and exit_code",
            example: "^external arg1 | complete",
            result: None,
        }]
    }

    fn pipe_redirection(&self) -> (Option<OutDest>, Option<OutDest>) {
        (Some(OutDest::PipeSeparate), Some(OutDest::PipeSeparate))
    }
}
