use nu_engine::get_full_help;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{IntoPipelineData, PipelineData, ShellError, Signature, Type, Value};

#[derive(Clone)]
pub struct Query;

impl Command for Query {
    fn name(&self) -> &str {
        "query"
    }

    fn usage(&self) -> &str {
        "Show all the query commands."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("query").input_output_types(vec![(Type::Nothing, Type::Nothing)])
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
                &Query.signature(),
                &Query.examples(),
                engine_state,
                stack,
                self.is_parser_keyword(),
            ),
            call.head,
        )
        .into_pipeline_data())
    }
}
