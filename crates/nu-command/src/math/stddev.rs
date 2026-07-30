use super::variance::{compute_variance as variance, values_unit};
use crate::math::utils::{
    NumericUnit, expand_range_input, run_with_function, run_with_function_and_cell_paths,
    wrap_unit_f64,
};
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
                (Type::List(Box::new(Type::Duration)), Type::Duration),
                (Type::List(Box::new(Type::Filesize)), Type::Filesize),
                (Type::Range, Type::Number),
                (Type::table(), Type::record()),
                (Type::record(), Type::record()),
            ])
            .switch(
                "sample",
                "Calculate sample standard deviation (i.e. using N-1 as the denominator).",
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
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
        let sample = call.has_flag(engine_state, stack, "sample")?;
        let mf = compute_stddev(sample);
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
        let mf = compute_stddev(sample);
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
                description: "Compute the standard deviation of a list of numbers.",
                example: "[1 2 3 4 5] | math stddev",
                result: Some(Value::test_float(std::f64::consts::SQRT_2)),
            },
            Example {
                description: "Compute the sample standard deviation of a list of numbers.",
                example: "[1 2 3 4 5] | math stddev --sample",
                result: Some(Value::test_float(1.5811388300841898)),
            },
            Example {
                description: "Compute the standard deviation of each column in a table.",
                example: "[[a b]; [1 2] [3 4]] | math stddev",
                result: Some(Value::test_record(record! {
                    "a" => Value::test_float(1.0),
                    "b" => Value::test_float(1.0),
                })),
            },
            Example {
                description: "Compute the standard deviation of list-valued columns in a record.",
                example: "{alice: [1 3], bob: [4 6]} | math stddev",
                result: Some(Value::test_record(record! {
                    "alice" => Value::test_float(1.0),
                    "bob" => Value::test_float(1.0),
                })),
            },
            Example {
                description: "Compute the standard deviation of a single column using a cell path.",
                example: "{alice: [1 3], bob: [4 6]} | math stddev alice",
                result: Some(Value::test_record(record! {
                    "alice" => Value::test_float(1.0),
                    "bob" => Value::list(
                        vec![Value::test_int(4), Value::test_int(6)],
                        Span::test_data(),
                    ),
                })),
            },
        ]
    }
}

pub fn compute_stddev(sample: bool) -> impl Fn(&[Value], Span, Span) -> Result<Value, ShellError> {
    move |values: &[Value], span: Span, head: Span| {
        let unit = values_unit(values, head)?;
        // variance() produces its own usable error, so we can use `?` to propagate the error.
        let variance = variance(sample)(values, span, head)?;
        let val_span = variance.span();
        let sqrt = match variance {
            Value::Float { val, .. } => val.sqrt(),
            Value::Int { val, .. } => (val as f64).sqrt(),
            other => return Ok(other),
        };
        // Duration/filesize keep their unit (stddev has the same unit as the inputs).
        // Numbers stay floats, matching the previous behavior.
        match unit {
            NumericUnit::Number => Ok(Value::float(sqrt, val_span)),
            other => Ok(wrap_unit_f64(other, sqrt, val_span)),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(MathStddev)
    }
}
