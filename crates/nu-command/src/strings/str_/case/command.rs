use nu_engine::get_full_help;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    IntoPipelineData, PipelineData, Signature, Value,
};

#[derive(Clone)]
pub struct Case;

impl Command for Case {
    fn name(&self) -> &str {
        "str"
    }

    fn signature(&self) -> Signature {
        Signature::build("str")
    }

    fn usage(&self) -> &str {
        "Converts strings into different kind of cases: camel, kebab, pascal, snake, and screaming snake."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        Ok(Value::String {
            val: get_full_help(&Case.signature(), &Case.examples(), engine_state),
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

        test_examples(Case {})
    }
}
