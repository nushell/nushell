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
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;

        if cfg!(windows) {
            CommandSys::new("cmd")
                .args(["/C", "cls"])
                .status()
                .map_err(|e| ShellError::IOErrorSpanned(e.to_string(), span))?;
        } else if cfg!(unix) {
            let mut cmd = CommandSys::new("/bin/sh");

            if let Some(Value::String { val, .. }) = stack.get_env_var(engine_state, "TERM") {
                cmd.env("TERM", val);
            }

            cmd.args(["-c", "clear"])
                .status()
                .map_err(|e| ShellError::IOErrorSpanned(e.to_string(), span))?;
        }

        Ok(Value::Nothing { span }.into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Clear the terminal",
            example: "clear",
            result: None,
        }]
    }
}
