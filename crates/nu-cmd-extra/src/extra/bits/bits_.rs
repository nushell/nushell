use nu_engine::{command_prelude::*, get_full_help};

#[derive(Clone)]
pub struct Bits;

impl Command for Bits {
    fn name(&self) -> &str {
        "bits"
    }

    fn signature(&self) -> Signature {
        Signature::build("bits")
            .category(Category::Bits)
            .input_output_types(vec![(Type::Nothing, Type::String)])
    }

    fn usage(&self) -> &str {
        "Various commands for working with bits."
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
                &Bits.signature(),
                &Bits.examples(),
                engine_state,
                stack,
                self.is_parser_keyword(),
            ),
            call.head,
        )
        .into_pipeline_data())
    }
}
