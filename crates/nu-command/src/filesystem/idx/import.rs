use super::state::restore_snapshot;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct IdxImport;

impl Command for IdxImport {
    fn name(&self) -> &str {
        "idx import"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "filepath",
                SyntaxShape::Filepath,
                "Path to a stored idx snapshot.",
            )
            .input_output_types(vec![(Type::Nothing, Type::record())])
            .category(Category::FileSystem)
    }

    fn description(&self) -> &str {
        "Import idx state from disk."
    }

    fn extra_description(&self) -> &str {
        "Reads a SQLite snapshot created by `idx export` and hydrates the runtime from stored rows. Watch mode is not restored from the snapshot."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Restore an idx index from a snapshot on disk",
            example: "idx import ~/my-index.db",
            result: None,
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let path: Spanned<String> = call.req(engine_state, stack, 0)?;
        let cwd = engine_state.cwd(Some(stack))?;
        let abs = nu_path::expand_path_with(path.item, cwd, true);
        Ok(PipelineData::value(
            restore_snapshot(abs.as_ref(), call.head)?,
            None,
        ))
    }
}
