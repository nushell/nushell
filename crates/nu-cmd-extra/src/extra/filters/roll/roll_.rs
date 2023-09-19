use nu_engine::get_full_help;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, IntoPipelineData, PipelineData, ShellError, Signature, Type, Value};

#[derive(Clone)]
pub struct Roll;

impl Command for Roll {
    fn name(&self) -> &str {
        "roll"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["rotate", "shift", "move"]
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .category(Category::Filters)
            .input_output_types(vec![(Type::Nothing, Type::String)])
    }

    fn usage(&self) -> &str {
        "Rolling commands for tables."
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
                &Roll.signature(),
                &Roll.examples(),
                engine_state,
                stack,
                self.is_parser_keyword(),
            ),
            call.head,
        )
        .into_pipeline_data())
    }
}
