use super::variance::compute_variance as variance;
use crate::math::utils::run_with_function;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, Span, Type, Value};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "math stddev"
    }

    fn signature(&self) -> Signature {
        Signature::build("math stddev")
            .input_output_types(vec![(Type::List(Box::new(Type::Number)), Type::Number)])
            .switch(
                "sample",
                "calculate sample standard deviation (i.e. using N-1 as the denominator)",
                Some('s'),
            )
            .category(Category::Math)
    }

    fn usage(&self) -> &str {
        "Returns the standard deviation of a list of numbers, or of each column in a table"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec![
            "SD",
            "standard",
            "deviation",
            "dispersion",
            "variation",
            "statistics",
        ]
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
                description: "Compute the standard deviation of a list of numbers",
                example: "[1 2 3 4 5] | math stddev",
                result: Some(Value::test_float(std::f64::consts::SQRT_2)),
            },
            Example {
                description: "Compute the sample standard deviation of a list of numbers",
                example: "[1 2 3 4 5] | math stddev -s",
                result: Some(Value::test_float(1.5811388300841898)),
            },
        ]
    }
}

pub fn compute_stddev(sample: bool) -> impl Fn(&[Value], Span, &Span) -> Result<Value, ShellError> {
    move |values: &[Value], span: Span, head: &Span| {
        let variance = variance(sample)(values, span, head);
        match variance {
            Ok(Value::Float { val, span }) => Ok(Value::Float {
                val: val.sqrt(),
                span,
            }),
            Ok(Value::Int { val, span }) => Ok(Value::Float {
                val: (val as f64).sqrt(),
                span,
            }),
            // variance() produces its own usable error, which can simply be propagated.
            Err(e) => Err(e),
            other => other,
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
