use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature};

#[derive(Clone)]
pub struct To;

impl Command for To {
    fn name(&self) -> &str {
        "to"
    }

    fn usage(&self) -> &str {
        "Translate structured data to a format"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("to").category(Category::Formats)
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, ShellError> {
        Ok(PipelineData::new(call.head))
    }
}
