use nu_engine::get_full_help;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, IntoPipelineData, PipelineData, ShellError, Signature, Value};

#[derive(Clone)]
pub struct From;

impl Command for From {
    fn name(&self) -> &str {
        "from"
    }

    fn usage(&self) -> &str {
        "Parse a string or binary data into structured data"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("from").category(Category::Formats)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, ShellError> {
        Ok(Value::String {
            val: get_full_help(&From.signature(), &From.examples(), engine_state, stack),
            span: call.head,
        }
        .into_pipeline_data())
    }
}
