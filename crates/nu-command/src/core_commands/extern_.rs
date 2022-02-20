use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, Signature, SyntaxShape};

#[derive(Clone)]
pub struct Extern;

impl Command for Extern {
    fn name(&self) -> &str {
        "extern"
    }

    fn usage(&self) -> &str {
        "Define a signature for an external command"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("extern")
            .required("def_name", SyntaxShape::String, "definition name")
            .required("params", SyntaxShape::Signature, "parameters")
            .category(Category::Core)
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        Ok(PipelineData::new(call.head))
    }
}
