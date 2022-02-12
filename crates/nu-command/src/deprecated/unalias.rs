use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, PipelineData, Signature,
};

#[derive(Clone)]
pub struct UnaliasDeprecated;

impl Command for UnaliasDeprecated {
    fn name(&self) -> &str {
        "unalias"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name()).category(Category::Deprecated)
    }

    fn usage(&self) -> &str {
        "Deprecated command"
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        Err(nu_protocol::ShellError::DeprecatedCommand(
            self.name().to_string(),
            "hide".to_string(),
            call.head,
        ))
    }
}
