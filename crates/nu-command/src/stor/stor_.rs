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

    fn usage(&self) -> &str {
        "Various commands for working with the in-memory sqlite database."
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
                &Stor.signature(),
                &Stor.examples(),
                engine_state,
                stack,
                self.is_parser_keyword(),
            ),
            call.head,
        )
        .into_pipeline_data())
    }
}
