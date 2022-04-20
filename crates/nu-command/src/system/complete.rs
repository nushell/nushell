use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Value,
};

#[derive(Clone)]
pub struct Complete;

impl Command for Complete {
    fn name(&self) -> &str {
        "complete"
    }

    fn signature(&self) -> Signature {
        Signature::build("complete").category(Category::System)
    }

    fn usage(&self) -> &str {
        "Complete the external piped in, collecting outputs and exit code"
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        match input {
            PipelineData::ExternalStream {
                stdout,
                stderr,
                exit_code,
                ..
            } => {
                let mut cols = vec![];
                let mut vals = vec![];

                if let Some(stdout) = stdout {
                    cols.push("stdout".to_string());
                    let stdout = stdout.into_bytes()?;
                    if let Ok(st) = String::from_utf8(stdout.item.clone()) {
                        vals.push(Value::String {
                            val: st,
                            span: stdout.span,
                        })
                    } else {
                        vals.push(Value::Binary {
                            val: stdout.item,
                            span: stdout.span,
                        })
                    }
                }

                if let Some(stderr) = stderr {
                    cols.push("stderr".to_string());
                    let stderr = stderr.into_bytes()?;
                    if let Ok(st) = String::from_utf8(stderr.item.clone()) {
                        vals.push(Value::String {
                            val: st,
                            span: stderr.span,
                        })
                    } else {
                        vals.push(Value::Binary {
                            val: stderr.item,
                            span: stderr.span,
                        })
                    };
                }

                if let Some(exit_code) = exit_code {
                    let mut v: Vec<_> = exit_code.collect();

                    if let Some(v) = v.pop() {
                        cols.push("exit_code".to_string());
                        vals.push(v);
                    }
                }

                Ok(Value::Record {
                    cols,
                    vals,
                    span: call.head,
                }
                .into_pipeline_data())
            }
            _ => Err(ShellError::GenericError(
                "Complete only works with external streams".to_string(),
                "complete only works on external streams".to_string(),
                Some(call.head),
                None,
                Vec::new(),
            )),
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Run the external completion",
            example: "^external arg1 | complete",
            result: None,
        }]
    }
}
