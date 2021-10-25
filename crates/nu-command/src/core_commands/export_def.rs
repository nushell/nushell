use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, EvaluationContext, Stack};
use nu_protocol::{PipelineData, Signature, SyntaxShape, Value};

#[derive(Clone)]
pub struct ExportDef;

impl Command for ExportDef {
    fn name(&self) -> &str {
        "export def"
    }

    fn usage(&self) -> &str {
        "Define a custom command and export it from a module"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("export def")
            .required("target", SyntaxShape::String, "definition name")
            .required("params", SyntaxShape::Signature, "parameters")
            .required(
                "block",
                SyntaxShape::Block(Some(vec![])),
                "body of the definition",
            )
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
