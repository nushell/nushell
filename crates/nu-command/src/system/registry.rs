use nu_engine::{command_prelude::*, get_full_help};

#[derive(Clone)]
pub struct Registry;

impl Command for Registry {
    fn name(&self) -> &str {
        "registry"
    }

    fn signature(&self) -> Signature {
        Signature::build("registry")
            .category(Category::System)
            .input_output_types(vec![(Type::Nothing, Type::String)])
    }

    fn description(&self) -> &str {
        "Various commands for interacting with the system registry (Windows only)."
    }

    fn extra_description(&self) -> &str {
        "You must use one of the following subcommands. Using this command as-is will only produce this help message."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> std::result::Result<PipelineData, ShellError> {
        Ok(Value::string(get_full_help(self, engine_state, stack), call.head).into_pipeline_data())
    }
}
