use nu_engine::CallExt;
use nu_protocol::ast::{Call, CellPath};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, IntoPipelineData, PipelineData, Signature, SyntaxShape, Value};

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
        Signature::build("get")
            .required(
                "cell_path",
                SyntaxShape::CellPath,
                "the cell path to the data",
            )
            .switch(
                "ignore-errors",
                "return nothing if path can't be found",
                Some('i'),
            )
            .category(Category::Filters)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let cell_path: CellPath = call.req(engine_state, stack, 0)?;
        let ignore_errors = call.has_flag("ignore-errors");

        let output = input
            .follow_cell_path(&cell_path.members, call.head)
            .map(|x| x.into_pipeline_data());

        if ignore_errors {
            match output {
                Ok(output) => Ok(output),
                Err(_) => Ok(Value::Nothing { span: call.head }.into_pipeline_data()),
            }
        } else {
            output
        }
    }
}
