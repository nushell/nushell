use super::state::store_snapshot;
use nu_engine::command_prelude::*;
#[derive(Clone)]
pub struct IdxExport;

impl Command for IdxExport {
    fn name(&self) -> &str {
        "idx export"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "filepath",
                SyntaxShape::Filepath,
                "Path where idx snapshot should be stored.",
            )
            .input_output_types(vec![(Type::Nothing, Type::record())])
            .category(Category::FileSystem)
    }

    fn description(&self) -> &str {
        "Persist idx state to disk."
    }

    fn extra_description(&self) -> &str {
        "The snapshot is stored as a SQLite database. Use `idx import` to reload it."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Save the current idx index to disk",
            example: "idx export ~/my-index.db",
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
            store_snapshot(abs.as_ref(), call.head)?,
            None,
        ))
    }
}
