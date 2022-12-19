use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, IntoPipelineData, PipelineData, Signature, Type, Value};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "math pi"
    }

    fn signature(&self) -> Signature {
        Signature::build("math pi")
            .input_output_types(vec![(Type::Any, Type::Float)])
            .category(Category::Math)
    }

    fn usage(&self) -> &str {
        "Returns the mathematical constant π."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["trigonometry", "constant"]
    }

    #[allow(clippy::approx_constant)]
    fn examples(&self) -> Vec<Example> {
        vec![Example {
            example: "math pi | math round --precision 2",
            description: "Get the first two decimal digits of π",
            result: Some(Value::test_float(3.14)),
        }]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        Ok(Value::float(std::f64::consts::PI, call.head).into_pipeline_data())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }
}
