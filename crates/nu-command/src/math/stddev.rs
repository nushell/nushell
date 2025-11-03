use super::variance::compute_variance as variance;
use crate::math::utils::run_with_function;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct MathStddev;

impl Command for MathStddev {
    fn name(&self) -> &str {
        "math stddev"
    }

    fn signature(&self) -> Signature {
        Signature::build("math stddev")
            .input_output_types(vec![
                (Type::List(Box::new(Type::Number)), Type::Number),
                (Type::Range, Type::Number),
                (Type::table(), Type::record()),
                (Type::record(), Type::record()),
            ])
            .switch(
                "sample",
                "calculate sample standard deviation (i.e. using N-1 as the denominator)",
                Some('s'),
            )
            .allow_variants_without_examples(true)
            .category(Category::Math)
    }

    fn description(&self) -> &str {
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

    fn is_const(&self) -> bool {
        true
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let sample = call.has_flag(engine_state, stack, "sample")?;
        let name = call.head;
        let span = input.span().unwrap_or(name);
        let input: PipelineData = match input.try_expand_range() {
            Err(_) => {
                return Err(ShellError::IncorrectValue {
                    msg: "Range must be bounded".to_string(),
                    val_span: span,
                    call_span: name,
                });
            }
            Ok(val) => val,
        };
        run_with_function(call, input, compute_stddev(sample))
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let sample = call.has_flag_const(working_set, "sample")?;
        let name = call.head;
        let span = input.span().unwrap_or(name);
        let input: PipelineData = match input.try_expand_range() {
            Err(_) => {
                return Err(ShellError::IncorrectValue {
                    msg: "Range must be bounded".to_string(),
                    val_span: span,
                    call_span: name,
                });
            }
            Ok(val) => val,
        };
        run_with_function(call, input, compute_stddev(sample))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Compute the standard deviation of a list of numbers",
                example: "[1 2 3 4 5] | math stddev",
                result: Some(Value::test_float(std::f64::consts::SQRT_2)),
            },
            Example {
                description: "Compute the sample standard deviation of a list of numbers",
                example: "[1 2 3 4 5] | math stddev --sample",
                result: Some(Value::test_float(1.5811388300841898)),
            },
            Example {
                description: "Compute the standard deviation of each column in a table",
                example: "[[a b]; [1 2] [3 4]] | math stddev",
                result: Some(Value::test_record(record! {
                    "a" => Value::test_int(1),
                    "b" => Value::test_int(1),
                })),
            },
        ]
    }
}

pub fn compute_stddev(sample: bool) -> impl Fn(&[Value], Span, Span) -> Result<Value, ShellError> {
    move |values: &[Value], span: Span, head: Span| {
        // variance() produces its own usable error, so we can use `?` to propagated the error.
        let variance = variance(sample)(values, span, head)?;
        let val_span = variance.span();
        match variance {
            Value::Float { val, .. } => Ok(Value::float(val.sqrt(), val_span)),
            Value::Int { val, .. } => Ok(Value::float((val as f64).sqrt(), val_span)),
            other => Ok(other),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(MathStddev {})
    }
}
