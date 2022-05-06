use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Example, Signature, Span, Value, IntoPipelineData};

#[derive(Clone)]
pub struct SeqChar;

impl Command for SeqChar {
    fn name(&self) -> &str {
        "seq char"
    }

    fn usage(&self) -> &str {
        "Print sequence of chars"
    }

    fn signature(&self) -> Signature {
        Signature::build("seq cha")
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "aaa",
            example: "seq char",
            result: None,
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &nu_protocol::ast::Call,
        input: nu_protocol::PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        Ok(Value::Bool {
            val: true,
            span: Span::test_data(),
        }.into_pipeline_data())
    }
}
