use crate::math::utils::{expand_range_input, run_with_function, run_with_function_and_cell_paths};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct MathVariance;

impl Command for MathVariance {
    fn name(&self) -> &str {
        "math variance"
    }

    fn signature(&self) -> Signature {
        Signature::build("math variance")
            .input_output_types(vec![
                (Type::List(Box::new(Type::Number)), Type::Number),
                (Type::Range, Type::Number),
                (Type::table(), Type::record()),
                (Type::record(), Type::record()),
            ])
            .switch(
                "sample",
                "Calculate sample variance (i.e. using N-1 as the denominator).",
                Some('s'),
            )
            .rest(
                "columns",
                SyntaxShape::CellPath,
                "The cell-paths/columns to operate on.",
            )
            .allow_variants_without_examples(true)
            .category(Category::Math)
    }

    fn description(&self) -> &str {
        "Returns the variance of a list of numbers or of each column in a table."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["deviation", "dispersion", "variation", "statistics"]
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
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
        let sample = call.has_flag(engine_state, stack, "sample")?;
        let mf = compute_variance(sample);
        if cell_paths.is_empty() {
            let input = expand_range_input(input, call.head)?;
            return run_with_function(call, input, mf);
        }
        run_with_function_and_cell_paths(call, input, cell_paths, engine_state.signals(), mf)
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let cell_paths: Vec<CellPath> = call.rest_const(working_set, 0)?;
        let sample = call.has_flag_const(working_set, "sample")?;
        let mf = compute_variance(sample);
        if cell_paths.is_empty() {
            let input = expand_range_input(input, call.head)?;
            return run_with_function(call, input, mf);
        }
        run_with_function_and_cell_paths(
            call,
            input,
            cell_paths,
            working_set.permanent().signals(),
            mf,
        )
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Get the variance of a list of numbers.",
                example: "[1 2 3 4 5] | math variance",
                result: Some(Value::test_float(2.0)),
            },
            Example {
                description: "Get the sample variance of a list of numbers.",
                example: "[1 2 3 4 5] | math variance --sample",
                result: Some(Value::test_float(2.5)),
            },
            Example {
                description: "Compute the variance of each column in a table.",
                example: "[[a b]; [1 2] [3 4]] | math variance",
                result: Some(Value::test_record(record! {
                    "a" => Value::test_int(1),
                    "b" => Value::test_int(1),
                })),
            },
            Example {
                description: "Compute the variance of list-valued columns in a record.",
                example: "{alice: [1 3], bob: [4 6]} | math variance",
                result: Some(Value::test_record(record! {
                    "alice" => Value::test_int(1),
                    "bob" => Value::test_int(1),
                })),
            },
            Example {
                description: "Compute the variance of a single column using a cell path.",
                example: "{alice: [1 3], bob: [4 6]} | math variance alice",
                result: Some(Value::test_record(record! {
                    "alice" => Value::test_int(1),
                    "bob" => Value::list(
                        vec![Value::test_int(4), Value::test_int(6)],
                        Span::test_data(),
                    ),
                })),
            },
        ]
    }
}

fn sum_of_squares(values: &[Value], span: Span) -> Result<Value, ShellError> {
    let n = Value::int(values.len() as i64, span);
    let mut sum_x = Value::int(0, span);
    let mut sum_x2 = Value::int(0, span);
    for value in values {
        let v = match &value {
            Value::Int { .. } | Value::Float { .. } => Ok(value),
            Value::Error { error, .. } => Err(*error.clone()),
            other => Err(ShellError::UnsupportedInput {
                msg: format!(
                    "Attempted to compute the sum of squares of a non-int, non-float value '{}' with a type of `{}`.",
                    other.coerce_string()?,
                    other.get_type()
                ),
                input: "value originates from here".into(),
                msg_span: span,
                input_span: value.span(),
            }),
        }?;
        let v_squared = &v.mul(span, v, span)?;
        sum_x2 = sum_x2.add(span, v_squared, span)?;
        sum_x = sum_x.add(span, v, span)?;
    }

    let sum_x_squared = sum_x.mul(span, &sum_x, span)?;
    let sum_x_squared_div_n = sum_x_squared.div(span, &n, span)?;

    let ss = sum_x2.sub(span, &sum_x_squared_div_n, span)?;

    Ok(ss)
}

pub fn compute_variance(
    sample: bool,
) -> impl Fn(&[Value], Span, Span) -> Result<Value, ShellError> {
    move |values: &[Value], span: Span, head: Span| {
        let n = if sample {
            values.len() - 1
        } else {
            values.len()
        };
        // sum_of_squares() needs the span of the original value, not the call head.
        let ss = sum_of_squares(values, span)?;
        let n = Value::int(n as i64, head);
        ss.div(head, &n, head)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(MathVariance)
    }
}
