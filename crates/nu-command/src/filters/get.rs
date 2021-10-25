use nu_engine::CallExt;
use nu_protocol::ast::{Call, CellPath};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{IntoPipelineData, PipelineData, Signature, SyntaxShape};

#[derive(Clone)]
pub struct Get;

impl Command for Get {
    fn name(&self) -> &str {
        "get"
    }

    fn usage(&self) -> &str {
        "Extract data using a cell path."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("get").required(
            "cell_path",
            SyntaxShape::CellPath,
            "the cell path to the data",
        )
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let cell_path: CellPath = call.req(engine_state, stack, 0)?;

        input
            .follow_cell_path(&cell_path.members)
            .map(|x| x.into_pipeline_data())
    }
}
