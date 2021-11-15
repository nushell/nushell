use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{PipelineData, Signature, SyntaxShape};

#[derive(Clone)]
pub struct ExportEnv;

impl Command for ExportEnv {
    fn name(&self) -> &str {
        "export env"
    }

    fn usage(&self) -> &str {
        "Export a block from a module that will be evaluated as an environment variable when imported."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("export env")
            .required(
                "name",
                SyntaxShape::String,
                "name of the environment variable",
            )
            .required(
                "block",
                SyntaxShape::Block(Some(vec![])),
                "body of the environment variable definition",
            )
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        //TODO: Add the env to stack
        Ok(PipelineData::new(call.head))
    }
}
