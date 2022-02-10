use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, Signature, SyntaxShape};

#[derive(Clone)]
pub struct Def;

impl Command for Def {
    fn name(&self) -> &str {
        "def"
    }

    fn usage(&self) -> &str {
        "Define a custom command"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("def")
            .required("name", SyntaxShape::String, "definition name")
            .required("params", SyntaxShape::Signature, "parameters")
            .required(
                "block",
                SyntaxShape::Block(Some(vec![])),
                "body of the definition",
            )
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
