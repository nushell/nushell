use super::state::init_runtime;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct IdxInit;

impl Command for IdxInit {
    fn name(&self) -> &str {
        "idx init"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("path", SyntaxShape::Directory, "Path to index.")
            .switch(
                "wait",
                "Block until the initial scan completes before returning.",
                Some('w'),
            )
            .input_output_types(vec![(Type::Nothing, Type::record())])
            .category(Category::FileSystem)
    }

    fn description(&self) -> &str {
        "Initialize the in-memory idx index for a path."
    }

    fn extra_description(&self) -> &str {
        "By default idx init returns immediately and indexing continues in the background. Use `idx status` to check when scanning completes. Pass `--wait` to block until the initial scan finishes."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Initialize idx for the current directory",
                example: "idx init .",
                result: None,
            },
            Example {
                description: "Initialize idx and wait for the initial scan to complete",
                example: "idx init . --wait",
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
        let path: Spanned<String> = call.req(engine_state, stack, 0)?;
        let wait = call.has_flag(engine_state, stack, "wait")?;
        let cwd = engine_state.cwd(Some(stack))?;
        let abs = nu_path::expand_path_with(path.item, cwd, true);
        // There is a functionality in fff-search to update the index via watch but it was non-trivial to get working
        // So, for now, let's always default to watch = false.
        let watch = false;
        let status = init_runtime(&abs, watch, wait, call.head)?;
        Ok(PipelineData::value(status.to_value(call.head), None))
    }
}
