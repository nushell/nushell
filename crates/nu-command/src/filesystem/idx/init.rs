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
            .switch(
                "no-watch",
                "Disable filesystem watching after the initial scan (watching is enabled by default).",
                None,
            )
            .switch(
                "no-content-indexing",
                "Disable file content indexing (content indexing is enabled by default).",
                None,
            )
            .switch(
                "follow-symlinks",
                "Whether to follow symlinks when indexing.",
                Some('f'),
            )
            .input_output_types(vec![(Type::Nothing, Type::record())])
            .category(Category::FileSystem)
    }

    fn description(&self) -> &str {
        "Initialize the in-memory idx index for a path."
    }

    fn extra_description(&self) -> &str {
        "By default idx init returns immediately and indexing continues in the background. Use `idx status` to check when scanning completes. Pass `--wait` to block until the initial scan finishes. Filesystem watching is enabled by default; pass `--no-watch` to disable it."
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
            Example {
                description: "Initialize idx without filesystem watching",
                example: "idx init . --no-watch",
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
        let no_watch = call.has_flag(engine_state, stack, "no-watch")?;
        let no_indexing = call.has_flag(engine_state, stack, "no-content-indexing")?;
        let follow = call.has_flag(engine_state, stack, "follow-symlinks")?;
        let cwd = engine_state.cwd(Some(stack))?;
        let abs = nu_path::expand_path_with(path.item, cwd, true);
        let watch = !no_watch;
        let content_indexing = !no_indexing;
        let status = init_runtime(&abs, watch, wait, follow, content_indexing, call.head)?;
        Ok(PipelineData::value(status.to_value(call.head), None))
    }
}
