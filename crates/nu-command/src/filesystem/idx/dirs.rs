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
            .optional(
                "query",
                SyntaxShape::String,
                "Optional fuzzy query to filter indexed directories.",
            )
            .input_output_types(vec![(Type::Nothing, Type::List(Box::new(Type::record())))])
            .category(Category::FileSystem)
    }

    fn description(&self) -> &str {
        "List indexed directories, or fuzzy-match directories by query."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "List all indexed directories",
                example: "idx dirs",
                result: None,
            },
            Example {
                description: "Fuzzy-match indexed directories by query",
                example: "idx dirs src",
                result: None,
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let query = call.opt::<String>(engine_state, stack, 0)?;
        let signals = engine_state.signals();
        stream_dirs(query, call.head, signals)
    }
}
