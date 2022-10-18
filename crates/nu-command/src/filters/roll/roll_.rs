use nu_engine::get_full_help;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, IntoPipelineData, PipelineData, ShellError, Signature, Value};

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
        Signature::build(self.name()).category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Rolling commands for tables"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(Value::String {
            val: get_full_help(&Roll.signature(), &Roll.examples(), engine_state, stack),
            span: call.head,
        }
        .into_pipeline_data())
    }
}
