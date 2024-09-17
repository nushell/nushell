use nu_engine::{command_prelude::*, get_full_help};

#[derive(Clone)]
pub struct From;

impl Command for From {
    fn name(&self) -> &str {
        "from"
    }

    fn description(&self) -> &str {
        "Parse a string or binary data into structured data."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("from")
            .category(Category::Formats)
            .input_output_types(vec![(Type::Nothing, Type::String)])
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
