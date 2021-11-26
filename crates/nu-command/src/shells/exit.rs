use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature};

/// Source a file for environment variables.
#[derive(Clone)]
pub struct Exit;

impl Command for Exit {
    fn name(&self) -> &str {
        "exit"
    }

    fn signature(&self) -> Signature {
        Signature::build("exit").category(Category::Shells)
    }

    fn usage(&self) -> &str {
        "Runs a script file in the current context."
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        _call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        //TODO: add more shell support

        std::process::exit(0);
    }
}
