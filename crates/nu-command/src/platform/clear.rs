use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Value,
};
use std::process::Command as CommandSys;

#[derive(Clone)]
pub struct Clear;

impl Command for Clear {
    fn name(&self) -> &str {
        "clear"
    }

    fn usage(&self) -> &str {
        "Clear the terminal."
    }

    fn signature(&self) -> Signature {
        Signature::build("clear").category(Category::Platform)
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        if cfg!(windows) {
            CommandSys::new("cmd")
                .args(["/C", "cls"])
                .status()
                .expect("failed to execute process");
        } else if cfg!(unix) {
            CommandSys::new("/bin/sh")
                .args(["-c", "clear"])
                .status()
                .expect("failed to execute process");
        }

        Ok(Value::Nothing { span: call.head }.into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Clear the terminal",
            example: "clear",
            result: None,
        }]
    }
}
