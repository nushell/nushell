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
        "Use one of the subcommands: init, status, find, search, export, import, drop, dirs, files. Watch mode keeps the index warm as files change; disable it when you only need a snapshot of the current tree."
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
