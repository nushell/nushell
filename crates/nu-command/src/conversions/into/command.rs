use nu_engine::get_full_help;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    IntoPipelineData, PipelineData, Signature, Value,
};

#[derive(Clone)]
pub struct Into;

impl Command for Into {
    fn name(&self) -> &str {
        "into"
    }

    fn signature(&self) -> Signature {
        Signature::build("into")
    }

    fn usage(&self) -> &str {
        "Apply into function."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        Ok(Value::String {
            val: get_full_help(&Into.signature(), &[], engine_state),
            span: call.head,
        }
        .into_pipeline_data())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Into {})
    }
}
