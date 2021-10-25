use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, EvaluationContext, Stack};
use nu_protocol::{PipelineData, Signature, SyntaxShape, Value};

#[derive(Clone)]
pub struct Use;

impl Command for Use {
    fn name(&self) -> &str {
        "use"
    }

    fn usage(&self) -> &str {
        "Use definitions from a module"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("use").required("pattern", SyntaxShape::String, "import pattern")
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        Ok(PipelineData::new())
    }
}
