use nu_engine::{command_prelude::*, get_full_help};

#[derive(Clone)]
pub struct Abbreviations;

impl Command for Abbreviations {
    fn name(&self) -> &str {
        "abbr"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .category(Category::Platform)
            .input_output_types(vec![(Type::Nothing, Type::String)])
    }

    fn description(&self) -> &str {
        "Abbreviations related commands."
    }

    fn extra_description(&self) -> &str {
        "You must use one of the following subcommands. Using this command as-is will only produce this help message."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["alias", "shorthand"]
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
