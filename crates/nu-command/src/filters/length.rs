use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{IntoPipelineData, PipelineData, Signature, Value};

#[derive(Clone)]
pub struct Length;

impl Command for Length {
    fn name(&self) -> &str {
        "length"
    }

    fn usage(&self) -> &str {
        "Count the number of elements in the input."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("length")
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        match input {
            PipelineData::Value(Value::Nothing { .. }) => Ok(Value::Int {
                val: 0,
                span: call.head,
            }
            .into_pipeline_data()),
            _ => Ok(Value::Int {
                val: input.count() as i64,
                span: call.head,
            }
            .into_pipeline_data()),
        }
    }
}
