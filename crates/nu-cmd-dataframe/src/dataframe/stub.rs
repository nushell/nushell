use nu_engine::{command_prelude::*, get_full_help};

#[derive(Clone)]
pub struct Dfr;

impl Command for Dfr {
    fn name(&self) -> &str {
        "dfr"
    }

    fn usage(&self) -> &str {
        "Operate with data in a dataframe format."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("dfr")
            .category(Category::Custom("dataframe".into()))
            .input_output_types(vec![(Type::Nothing, Type::String)])
    }

    fn extra_usage(&self) -> &str {
        "You must use one of the following subcommands. Using this command as-is will only produce this help message."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(Value::string(
            get_full_help(
                &Dfr.signature(),
                &Dfr.examples(),
                engine_state,
                stack,
                self.is_parser_keyword(),
            ),
            call.head,
        )
        .into_pipeline_data())
    }
}
