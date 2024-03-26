use nu_engine::{command_prelude::*, get_full_help};

#[derive(Clone)]
pub struct Str;

impl Command for Str {
    fn name(&self) -> &str {
        "str"
    }

    fn signature(&self) -> Signature {
        Signature::build("str")
            .category(Category::Strings)
            .input_output_types(vec![(Type::Nothing, Type::String)])
    }

    fn usage(&self) -> &str {
        "Various commands for working with string data."
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
                &Str.signature(),
                &Str.examples(),
                engine_state,
                stack,
                self.is_parser_keyword(),
            ),
            call.head,
        )
        .into_pipeline_data())
    }
}
