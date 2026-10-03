use crate::math::utils::{
    NumericUnit, expand_range_input, run_with_function, run_with_function_and_cell_paths,
    to_unit_f64, variance_denominator,
};
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
                (Type::List(Box::new(Type::Duration)), Type::Number),
                (Type::List(Box::new(Type::Filesize)), Type::Number),
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

    fn extra_description(&self) -> &str {
        "For filesize and duration inputs, variance is computed in base units \
         (bytes and nanoseconds) and returned as a plain number. There is no \
         squared unit type in Nushell, so the result is the variance of the \
         underlying byte or nanosecond values (B² or ns²), not of the display \
         unit used when the values were written."
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
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let cell_paths: Vec<CellPath> = call.rest_const(working_set, stack, 0)?;
        let sample = call.has_flag_const(working_set, stack, "sample")?;
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
            Example {
                // 1KB=1000B, 3KB=3000B; population variance is 1_000_000 (B²), not 1 (KB²).
                description: "Variance of filesizes is a number of base units squared (bytes²).",
                example: "[1KB 3KB] | math variance",
                result: Some(Value::test_float(1_000_000.0)),
            },
            Example {
                // Large, close values: the two-pass formula avoids the
                // catastrophic cancellation that made the old single-pass
                // formula return 0.0 (or even a negative value) here.
                description: "Variance of large, close values is computed accurately.",
                example: "[506250000.0 506250001.0] | math variance",
                result: Some(Value::test_float(0.25)),
            },
        ]
    }
}

/// Numerically-stable two-pass variance in `f64` base units.
///
/// All numeric types (int/float/duration/filesize) are converted to `f64` and
/// the mean-deviation form `Σ(x − mean)² / n` is used. This avoids the
/// catastrophic cancellation of the single-pass `Σx² − (Σx)²/n` formula, which
/// loses precision (and can even go negative) when values are large but close
/// together. The result is a plain `f64`: squared base units (B² / ns²) for
/// filesize/duration, or a plain number for int/float.
fn variance_unit_f64(
    values: &[Value],
    sample: bool,
    span: Span,
    head: Span,
) -> Result<f64, ShellError> {
    let mut nums = Vec::with_capacity(values.len());
    for value in values {
        let (_, n) = to_unit_f64(value, head)?;
        nums.push(n);
    }
    let denom = variance_denominator(nums.len(), sample, head, span)? as f64;
    let mean = nums.iter().sum::<f64>() / nums.len() as f64;
    let ss = nums
        .iter()
        .map(|x| {
            let d = x - mean;
            d * d
        })
        .sum::<f64>();
    Ok(ss / denom)
}

pub fn compute_variance(
    sample: bool,
) -> impl Fn(&[Value], Span, Span) -> Result<Value, ShellError> {
    move |values: &[Value], span: Span, head: Span| {
        // Validate that all values share one numeric unit (int/float may mix;
        // duration/filesize must be homogeneous).
        let _ = values_unit(values, head)?;
        // Two-pass f64 variance for all units (numerically stable).
        let var = variance_unit_f64(values, sample, span, head)?;
        Ok(Value::float(var, span))
    }
}

/// Determine the common unit of a value list for re-wrapping stddev results.
pub fn values_unit(values: &[Value], head: Span) -> Result<NumericUnit, ShellError> {
    let mut unit = NumericUnit::Number;
    for (i, value) in values.iter().enumerate() {
        let (this_unit, _) = to_unit_f64(value, head)?;
        if i == 0 {
            unit = this_unit;
            continue;
        }
        // int and float may mix; duration/filesize must stay homogeneous.
        match (unit, this_unit) {
            (NumericUnit::Number, NumericUnit::Number) => {}
            (a, b) if a == b => {}
            (a, _) => {
                return Err(ShellError::OnlySupportsThisInputType {
                    exp_input_type: a.as_str().into(),
                    wrong_type: value.get_type().to_string(),
                    dst_span: head,
                    src_span: value.span(),
                });
            }
        }
    }
    Ok(unit)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(MathVariance)
    }
}
