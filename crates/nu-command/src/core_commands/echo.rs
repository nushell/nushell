use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{PipelineData, ShellError, Signature, SyntaxShape};
//TODO: add Example

#[derive(Clone)]
pub struct Echo;

impl Command for Echo {
    fn name(&self) -> &str {
        "echo"
    }

    fn usage(&self) -> &str {
        "Echo the arguments back to the user."
    }

    fn signature(&self) -> Signature {
        Signature::build("echo").rest("rest", SyntaxShape::Any, "the values to echo")
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        _call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(input)
    }
    //TODO: implement fn examples(&self) -> Vec<Example>
}
