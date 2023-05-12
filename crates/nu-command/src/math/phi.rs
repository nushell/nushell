use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Type, Value,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "math phi"
    }

    fn signature(&self) -> Signature {
        Signature::build("math phi")
            .input_output_types(vec![(Type::Any, Type::Float)])
            .category(Category::Math)
    }

    fn usage(&self) -> &str {
        "Returns the golden ratio φ. ( (1 + sqrt(5) ) / 2 )"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["golden", "ratio", "constant"]
    }

    #[allow(clippy::approx_constant)]
    fn examples(&self) -> Vec<Example> {
        vec![Example {
            example: "math phi | math round --precision 3",
            description: "Get the first two decimal digits of φ",
            result: Some(Value::test_float(1.618)),
        }]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(Value::float(std::f64::consts::PHI, call.head).into_pipeline_data())
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
