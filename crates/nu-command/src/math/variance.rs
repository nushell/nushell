use crate::math::utils::run_with_function;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, Span, Type, Value};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "math variance"
    }

    fn signature(&self) -> Signature {
        Signature::build("math variance")
            .input_output_types(vec![(Type::List(Box::new(Type::Number)), Type::Number)])
            .switch(
                "sample",
                "calculate sample variance (i.e. using N-1 as the denominator)",
                Some('s'),
            )
            .category(Category::Math)
    }

    fn usage(&self) -> &str {
        "Returns the variance of a list of numbers or of each column in a table"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["deviation", "dispersion", "variation", "statistics"]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let sample = call.has_flag("sample");
        run_with_function(call, input, compute_variance(sample))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get the variance of a list of numbers",
                example: "echo [1 2 3 4 5] | math variance",
                result: Some(Value::Float {
                    val: 2.0,
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Get the sample variance of a list of numbers",
                example: "[1 2 3 4 5] | math variance -s",
                result: Some(Value::Float {
                    val: 2.5,
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

fn sum_of_squares(values: &[Value], span: &Span) -> Result<Value, ShellError> {
    let n = Value::Int {
        val: values.len() as i64,
        span: *span,
    };
    let mut sum_x = Value::Int {
        val: 0,
        span: *span,
    };
    let mut sum_x2 = Value::Int {
        val: 0,
        span: *span,
    };
    for value in values {
        let v = match &value {
            Value::Int { .. }
            | Value::Float { .. } => {
                Ok(value)
            },
            _ => Err(ShellError::UnsupportedInput(
                    "Attempted to compute the sum of squared values of a value that cannot be summed or squared.".to_string(),
                    value.span().unwrap_or(*span),
                ))
        }?;
        let v_squared = &v.mul(*span, v, *span)?;
        sum_x2 = sum_x2.add(*span, v_squared, *span)?;
        sum_x = sum_x.add(*span, v, *span)?;
    }

    let sum_x_squared = sum_x.mul(*span, &sum_x, *span)?;
    let sum_x_squared_div_n = sum_x_squared.div(*span, &n, *span)?;

    let ss = sum_x2.sub(*span, &sum_x_squared_div_n, *span)?;

    Ok(ss)
}

pub fn compute_variance(sample: bool) -> impl Fn(&[Value], &Span) -> Result<Value, ShellError> {
    move |values: &[Value], span: &Span| {
        let n = if sample {
            values.len() - 1
        } else {
            values.len()
        };
        let sum_of_squares = sum_of_squares(values, span);
        let ss = match sum_of_squares {
            Err(ShellError::UnsupportedInput(_, err_span)) => Err(ShellError::UnsupportedInput(
                "Attempted to compute the variance with an item that cannot be used for that."
                    .to_string(),
                err_span,
            )),
            other => other,
        }?;
        let n = Value::Int {
            val: n as i64,
            span: *span,
        };
        ss.div(*span, &n, *span)
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
