use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EvaluationContext};
use nu_protocol::{PipelineData, ShellError, Signature, Value};

#[derive(Clone)]
pub struct From;

impl Command for From {
    fn name(&self) -> &str {
        "from"
    }

    fn usage(&self) -> &str {
        "Parse a string or binary data into structured data"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("from")
    }

    fn run(
        &self,
        _context: &EvaluationContext,
        _call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, ShellError> {
        Ok(PipelineData::new())
    }
}
