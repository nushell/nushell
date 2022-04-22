use nu_engine::get_full_help;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, IntoPipelineData, PipelineData, ShellError, Signature, Value,
};

#[derive(Clone)]
pub struct Query;

impl Command for Query {
    fn name(&self) -> &str {
        "query"
    }

    fn signature(&self) -> Signature {
        Signature::build("query").category(Category::Query)
    }

    fn usage(&self) -> &str {
        "Commands for querying data"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        {
            let head = call.head;

            Ok(Value::String {
                val: get_full_help(&Query.signature(), &Query.examples(), engine_state, stack),
                span: head,
            }
            .into_pipeline_data())
        }
    }
}
