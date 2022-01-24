use nu_engine::get_full_help;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, IntoPipelineData, PipelineData, ShellError, Signature, Value};

#[derive(Clone)]
pub struct Hash;

impl Command for Hash {
    fn name(&self) -> &str {
        "hash"
    }

    fn signature(&self) -> Signature {
        Signature::build("hash").category(Category::Hash)
    }

    fn usage(&self) -> &str {
        "Apply hash function."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(Value::String {
            val: get_full_help(&Self.signature(), &Self.examples(), engine_state, stack),
            span: call.head,
        }
        .into_pipeline_data())
    }
}
