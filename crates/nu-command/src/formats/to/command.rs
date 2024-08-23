use nu_engine::{command_prelude::*, get_full_help};

#[derive(Clone)]
pub struct To;

impl Command for To {
    fn name(&self) -> &str {
        "to"
    }

    fn description(&self) -> &str {
        "Translate structured data to a format."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("to")
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
