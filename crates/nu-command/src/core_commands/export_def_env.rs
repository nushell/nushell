use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, Signature, SyntaxShape};

#[derive(Clone)]
pub struct ExportDefEnv;

impl Command for ExportDefEnv {
    fn name(&self) -> &str {
        "export def_env"
    }

    fn usage(&self) -> &str {
        "Define a custom command that participates in the environment and export it from a module"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("export def_env")
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
