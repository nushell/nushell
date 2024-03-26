use nu_engine::{command_prelude::*, get_full_help};

#[derive(Clone)]
pub struct Url;

impl Command for Url {
    fn name(&self) -> &str {
        "url"
    }

    fn signature(&self) -> Signature {
        Signature::build("url")
            .input_output_types(vec![(Type::Nothing, Type::String)])
            .category(Category::Network)
    }

    fn usage(&self) -> &str {
        "Various commands for working with URLs."
    }

    fn extra_usage(&self) -> &str {
        "You must use one of the following subcommands. Using this command as-is will only produce this help message."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["network", "parse"]
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
                &Url.signature(),
                &Url.examples(),
                engine_state,
                stack,
                self.is_parser_keyword(),
            ),
            call.head,
        )
        .into_pipeline_data())
    }
}
