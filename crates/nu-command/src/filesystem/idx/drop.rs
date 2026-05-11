use super::state::drop_runtime;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct IdxDrop;

impl Command for IdxDrop {
    fn name(&self) -> &str {
        "idx drop"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![(Type::Nothing, Type::record())])
            .category(Category::FileSystem)
    }

    fn description(&self) -> &str {
        "Drop the current idx runtime from memory."
    }

    fn extra_description(&self) -> &str {
        "Use this when you want to free the in-memory index completely before reinitializing or restoring a different snapshot."
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(PipelineData::value(drop_runtime(call.head)?, None))
    }
}
