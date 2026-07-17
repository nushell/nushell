use nu_engine::{command_prelude::*, get_full_help};

#[derive(Clone)]
pub struct Idx;

impl Command for Idx {
    fn name(&self) -> &str {
        "idx"
    }

    fn signature(&self) -> Signature {
        Signature::build("idx")
            .input_output_types(vec![(Type::Nothing, Type::String)])
            .category(Category::FileSystem)
    }

    fn description(&self) -> &str {
        "Manage in-memory file index state."
    }

    fn extra_description(&self) -> &str {
        "Use one of the subcommands: init, status, find, search, watch, export, import, drop, dirs, files. \
By default `idx init` enables filesystem watching so the index stays warm as files change; pass `--no-watch` for a static snapshot. \
Use `idx watch` to stream change events from that live index into a pipeline (distinct from the plain `watch` command)."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(Value::string(
            get_full_help(self, engine_state, stack, call.head),
            call.head,
        )
        .into_pipeline_data())
    }
}
