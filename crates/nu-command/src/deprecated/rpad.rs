use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::Category;
use nu_protocol::{PipelineData, ShellError, Signature};

#[derive(Clone)]
pub struct RPadDeprecated;

impl Command for RPadDeprecated {
    fn name(&self) -> &str {
        "str rpad"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name()).category(Category::Deprecated)
    }

    fn usage(&self) -> &str {
        "Deprecated command."
    }

    fn run(
        &self,
        _: &EngineState,
        _: &mut Stack,
        call: &Call,
        _: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Err(nu_protocol::ShellError::DeprecatedCommand(
            self.name().to_string(),
            "fill".to_owned(),
            call.head,
        ))
    }
}
