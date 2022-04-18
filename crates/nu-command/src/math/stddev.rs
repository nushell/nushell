use super::variance::compute_variance as variance;
use crate::math::utils::run_with_function;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, Span, Value};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "math stddev"
    }

    fn signature(&self) -> Signature {
        Signature::build("math stddev")
            .switch("sample", "calculate sample standard deviation", Some('s'))
            .category(Category::Math)
    }

    fn usage(&self) -> &str {
        "Finds the stddev of a list of numbers or tables"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["SD", "standard", "deviation", "dispersion", "variation"]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let sample = call.has_flag("sample");
        run_with_function(call, input, compute_stddev(sample))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get the stddev of a list of numbers",
                example: "[1 2 3 4 5] | math stddev",
                result: Some(Value::Float {
                    val: std::f64::consts::SQRT_2,
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Get the sample stddev of a list of numbers",
                example: "[1 2 3 4 5] | math stddev -s",
                result: Some(Value::Float {
                    val: 1.5811388300841898,
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

pub fn compute_stddev(sample: bool) -> impl Fn(&[Value], &Span) -> Result<Value, ShellError> {
    move |values: &[Value], span: &Span| {
        let variance = variance(sample)(values, span);
        match variance {
            Ok(Value::Float { val, span }) => Ok(Value::Float { val: val.sqrt(), span }),
            Ok(Value::Int { val, span }) => Ok(Value::Float { val: (val as f64).sqrt(), span }),
            Err(ShellError::UnsupportedInput(_, err_span)) => Err(ShellError::UnsupportedInput(
                    "Attempted to compute the standard deviation with an item that cannot be used for that.".to_string(),
                    err_span,
                )),
            other => other
        }
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
