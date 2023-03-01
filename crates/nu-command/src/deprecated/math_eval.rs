use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "math eval"
    }

    fn signature(&self) -> Signature {
        Signature::build("math eval")
            .rest(
                "ignored",
                SyntaxShape::Any,
                "arguments to deprecated command are ignored",
            )
            .category(Category::Deprecated)
    }

    fn usage(&self) -> &str {
        "Deprecated command."
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Err(ShellError::DeprecatedCommand(
            self.name().to_string(),
            "math <subcommands>".to_string(),
            call.head,
        ))
    }
}
