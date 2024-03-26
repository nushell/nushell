use nu_engine::{command_prelude::*, get_full_help};

#[derive(Clone)]
pub struct MathCommand;

impl Command for MathCommand {
    fn name(&self) -> &str {
        "math"
    }

    fn signature(&self) -> Signature {
        Signature::build("math")
            .category(Category::Math)
            .input_output_types(vec![(Type::Nothing, Type::String)])
    }

    fn usage(&self) -> &str {
        "Use mathematical functions as aggregate functions on a list of numbers or tables."
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
                &MathCommand.signature(),
                &MathCommand.examples(),
                engine_state,
                stack,
                self.is_parser_keyword(),
            ),
            call.head,
        )
        .into_pipeline_data())
    }
}
