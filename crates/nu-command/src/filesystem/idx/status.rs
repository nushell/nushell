use super::state::current_status;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct IdxStatus;

impl Command for IdxStatus {
    fn name(&self) -> &str {
        "idx status"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![(Type::Nothing, Type::record())])
            .category(Category::FileSystem)
    }

    fn description(&self) -> &str {
        "Show status information for the global in-memory idx runtime."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Show the current idx runtime status",
            example: "idx status",
            result: None,
        }]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(PipelineData::value(
            current_status(None).to_value(call.head),
            None,
        ))
    }
}
