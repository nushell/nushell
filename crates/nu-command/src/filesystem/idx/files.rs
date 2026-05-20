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
                "path",
                SyntaxShape::String,
                "Optional path to lookup in index.",
            )
            .input_output_types(vec![(Type::Nothing, Type::List(Box::new(Type::record())))])
            .category(Category::FileSystem)
    }

    fn description(&self) -> &str {
        "List indexed files, or lookup a specific indexed path."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "List all indexed files",
                example: "idx files",
                result: None,
            },
            Example {
                description: "Lookup a specific file path in the index",
                example: "idx files src/main.rs",
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
        let path = call.opt::<String>(engine_state, stack, 0)?;
        let signals = engine_state.signals();
        stream_files(path, call.head, signals)
    }
}
