use nu_engine::{command_prelude::*, get_full_help};

#[derive(Clone)]
pub struct Stor;

impl Command for Stor {
    fn name(&self) -> &str {
        "stor"
    }

    fn signature(&self) -> Signature {
        Signature::build("stor")
            .category(Category::Database)
            .input_output_types(vec![(Type::Nothing, Type::String)])
    }

    fn description(&self) -> &str {
        "Various commands for working with the in-memory sqlite database."
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
    ) -> Result<PipelineData, ShellError> {
        Ok(Value::string(get_full_help(self, engine_state, stack), call.head).into_pipeline_data())
    }
}
