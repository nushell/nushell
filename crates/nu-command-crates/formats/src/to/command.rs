use nu_engine::get_full_help;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, IntoPipelineData, PipelineData, ShellError, Signature, Value};

#[derive(Clone)]
pub struct To;

impl Command for To {
    fn name(&self) -> &str {
        "to"
    }

    fn usage(&self) -> &str {
        "Translate structured data to a format"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("to").category(Category::Formats)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, ShellError> {
        Ok(Value::String {
            val: get_full_help(&To.signature(), &To.examples(), engine_state, stack),
            span: call.head,
        }
        .into_pipeline_data())
    }
}
