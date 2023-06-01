use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Type, Value,
};

#[allow(clippy::excessive_precision)]
/// The Euler-Mascheroni constant (γ)
pub const EGAMMA: f64 = 0.577215664901532860606512090082402431_f64;

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "math egamma"
    }

    fn signature(&self) -> Signature {
        Signature::build("math egamma")
            .input_output_types(vec![(Type::Any, Type::Float)])
            .category(Category::Math)
    }

    fn usage(&self) -> &str {
        "Returns the Euler–Mascheroni constant γ. ( 1 | math egamma)."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["euler", "constant", "gamma"]
    }

    #[allow(clippy::approx_constant)]
    fn examples(&self) -> Vec<Example> {
        vec![Example {
            example: "math egamma | math round --precision 3",
            description: "Get the first three decimal digits of γ",
            result: Some(Value::test_float(0.577)),
        }]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // TODO: replace with std::f64::consts::EGAMMA when available https://github.com/rust-lang/rust/issues/103883
        Ok(Value::float(EGAMMA, call.head).into_pipeline_data())
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
