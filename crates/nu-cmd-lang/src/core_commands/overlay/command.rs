use nu_engine::{command_prelude::*, get_full_help};
use nu_protocol::engine::CommandType;

#[derive(Clone)]
pub struct Overlay;

impl Command for Overlay {
    fn name(&self) -> &str {
        "overlay"
    }

    fn signature(&self) -> Signature {
        Signature::build("overlay")
            .category(Category::Core)
            .input_output_types(vec![(Type::Nothing, Type::String)])
    }

    fn description(&self) -> &str {
        "Commands for manipulating overlays."
    }

    fn extra_description(&self) -> &str {
        r#"This command is a parser keyword. For details, check:
  https://www.nushell.sh/book/thinking_in_nu.html

  You must use one of the following subcommands. Using this command as-is will only produce this help message."#
    }

    fn command_type(&self) -> CommandType {
        CommandType::Keyword
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
