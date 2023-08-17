use super::variance::compute_variance as variance;
use crate::math::utils::run_with_function;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SpannedValue, Type,
};

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
        "Returns the standard deviation of a list of numbers, or of each column in a table."
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
    ) -> Result<PipelineData, ShellError> {
        let sample = call.has_flag("sample");
        run_with_function(call, input, compute_stddev(sample))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Compute the standard deviation of a list of numbers",
                example: "[1 2 3 4 5] | math stddev",
                result: Some(SpannedValue::test_float(std::f64::consts::SQRT_2)),
            },
            Example {
                description: "Compute the sample standard deviation of a list of numbers",
                example: "[1 2 3 4 5] | math stddev -s",
                result: Some(SpannedValue::test_float(1.5811388300841898)),
            },
        ]
    }
}

pub fn compute_stddev(
    sample: bool,
) -> impl Fn(&[SpannedValue], Span, Span) -> Result<SpannedValue, ShellError> {
    move |values: &[SpannedValue], span: Span, head: Span| {
        let variance = variance(sample)(values, span, head);
        match variance {
            Ok(SpannedValue::Float { val, span }) => Ok(SpannedValue::Float {
                val: val.sqrt(),
                span,
            }),
            Ok(SpannedValue::Int { val, span }) => Ok(SpannedValue::Float {
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
