use nu_engine::{command_prelude::*, get_full_help};

#[derive(Clone)]
pub struct Http;

impl Command for Http {
    fn name(&self) -> &str {
        "http"
    }

    fn signature(&self) -> Signature {
        Signature::build("http")
            .input_output_types(vec![(Type::Nothing, Type::String)])
            .category(Category::Network)
    }

    fn description(&self) -> &str {
        "Various commands for working with http methods."
    }

    fn extra_description(&self) -> &str {
        "You must use one of the following subcommands. Using this command as-is will only produce this help message."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec![
            "network", "fetch", "pull", "request", "download", "curl", "wget",
        ]
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
