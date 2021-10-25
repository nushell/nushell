use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{PipelineData, Signature, SyntaxShape};

#[derive(Clone)]
pub struct Hide;

impl Command for Hide {
    fn name(&self) -> &str {
        "hide"
    }

    fn usage(&self) -> &str {
        "Hide definitions in the current scope"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("hide").required("pattern", SyntaxShape::String, "import pattern")
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        _call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        Ok(PipelineData::new())
    }
}
