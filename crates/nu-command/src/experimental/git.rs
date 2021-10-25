use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, EvaluationContext, Stack};
use nu_protocol::{IntoPipelineData, PipelineData, Signature, Value};

#[derive(Clone)]
pub struct Git;

impl Command for Git {
    fn name(&self) -> &str {
        "git"
    }

    fn usage(&self) -> &str {
        "Run a block"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("git")
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        use std::process::Command as ProcessCommand;
        use std::process::Stdio;

        let proc = ProcessCommand::new("git").stdout(Stdio::piped()).spawn();

        match proc {
            Ok(child) => {
                match child.wait_with_output() {
                    Ok(val) => {
                        let result = val.stdout;

                        Ok(Value::String {
                            val: String::from_utf8_lossy(&result).to_string(),
                            span: call.head,
                        }
                        .into_pipeline_data())
                    }
                    Err(_err) => {
                        // FIXME: Move this to an external signature and add better error handling
                        Ok(PipelineData::new())
                    }
                }
            }
            Err(_err) => {
                // FIXME: Move this to an external signature and add better error handling
                Ok(PipelineData::new())
            }
        }
    }
}
