use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, Type};

#[derive(Clone)]
pub struct Continue;

impl Command for Continue {
    fn name(&self) -> &str {
        "continue"
    }

    fn usage(&self) -> &str {
        "Continue a loop from the next iteration."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("continue")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .category(Category::Core)
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Err(ShellError::Continue { span: call.head })
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Continue a loop from the next iteration",
            example: r#"for i in 1..10 { if $i == 5 { continue }; print $i }"#,
            result: None,
        }]
    }
}
