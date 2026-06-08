use super::state::stream_files;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct IdxFiles;

impl Command for IdxFiles {
    fn name(&self) -> &str {
        "idx files"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .optional(
                "query",
                SyntaxShape::String,
                "Optional fuzzy query to filter indexed files.",
            )
            .input_output_types(vec![(Type::Nothing, Type::List(Box::new(Type::record())))])
            .category(Category::FileSystem)
    }

    fn description(&self) -> &str {
        "List indexed files, or fuzzy-match files by query."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "List all indexed files",
                example: "idx files",
                result: None,
            },
            Example {
                description: "Fuzzy-match indexed files by query",
                example: "idx files main",
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
        stream_files(query, call.head, signals)
    }
}
