use super::state::stream_dirs;
use nu_engine::command_prelude::*;
#[derive(Clone)]
pub struct IdxDirs;

impl Command for IdxDirs {
    fn name(&self) -> &str {
        "idx dirs"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![(Type::Nothing, Type::List(Box::new(Type::record())))])
            .category(Category::FileSystem)
    }

    fn description(&self) -> &str {
        "List indexed directories from idx state."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "List all indexed directories",
            example: "idx dirs",
            result: None,
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let signals = engine_state.signals();
        stream_dirs(call.head, signals)
    }
}
