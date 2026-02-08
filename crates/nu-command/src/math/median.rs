use crate::math::{avg::average, utils::run_with_function};
use nu_engine::command_prelude::*;
use std::cmp::Ordering;

#[derive(Clone)]
pub struct MathMedian;

impl Command for MathMedian {
    fn name(&self) -> &str {
        "math median"
    }

    fn signature(&self) -> Signature {
        Signature::build("math median")
            .input_output_types(vec![
                (Type::List(Box::new(Type::Number)), Type::Number),
                (Type::List(Box::new(Type::Duration)), Type::Duration),
                (Type::List(Box::new(Type::Filesize)), Type::Filesize),
                (Type::Range, Type::Number),
                (Type::table(), Type::record()),
                (Type::record(), Type::record()),
            ])
            .allow_variants_without_examples(true)
            .category(Category::Math)
    }

    fn description(&self) -> &str {
        "Computes the median of a list of numbers."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["middle", "statistics"]
    }

    fn is_const(&self) -> bool {
        true
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        run_with_function(call, input, median)
    }

    fn run_const(
        &self,
        _working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        run_with_function(call, input, median)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Compute the median of a list of numbers",
                example: "[3 8 9 12 12 15] | math median",
                result: Some(Value::test_float(10.5)),
            },
            Example {
                description: "Compute the medians of the columns of a table",
                example: "[{a: 1 b: 3} {a: 2 b: -1} {a: -3 b: 5}] | math median",
                result: Some(Value::test_record(record! {
                    "a" => Value::test_int(1),
                    "b" => Value::test_int(3),
                })),
            },
            Example {
                description: "Find the median of a list of file sizes",
                example: "[5KB 10MB 200B] | math median",
                result: Some(Value::test_filesize(5 * 1_000)),
            },
        ]
    }
}

enum Pick {
    MedianAverage,
    Median,
}

pub fn median(values: &[Value], span: Span, head: Span) -> Result<Value, ShellError> {
    let mut sorted = values
        .iter()
        .filter(|x| !x.as_float().is_ok_and(f64::is_nan))
        .collect::<Vec<_>>();

    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));

    let take = if sorted.len().is_multiple_of(2) {
        Pick::MedianAverage
    } else {
        Pick::Median
    };

    match take {
        Pick::Median => {
            let idx = sorted.len() / 2;
            Ok(sorted
                .get(idx)
                .ok_or_else(|| ShellError::UnsupportedInput {
                    msg: "Empty input".to_string(),
                    input: "value originates from here".into(),
                    msg_span: head,
                    input_span: span,
                })?
                .to_owned()
                .to_owned())
        }
        Pick::MedianAverage => {
            let idx_end = sorted.len() / 2;
            let idx_start = idx_end - 1;

            let left = sorted
                .get(idx_start)
                .ok_or_else(|| ShellError::UnsupportedInput {
                    msg: "Empty input".to_string(),
                    input: "value originates from here".into(),
                    msg_span: head,
                    input_span: span,
                })?
                .to_owned()
                .to_owned();

            let right = sorted
                .get(idx_end)
                .ok_or_else(|| ShellError::UnsupportedInput {
                    msg: "Empty input".to_string(),
                    input: "value originates from here".into(),
                    msg_span: head,
                    input_span: span,
                })?
                .to_owned()
                .to_owned();

            average(&[left, right], span, head)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(MathMedian {})
    }

    #[test]
    fn test_median_with_nan_values() {
        // Test case: [NaN, NaN, 1.0, 2.0, 3.0, 4.0]
        // values.len() = 6, sorted = [1.0, 2.0, 3.0, 4.0], sorted.len() = 4
        // Correct median of [1.0, 2.0, 3.0, 4.0] is (2.0 + 3.0) / 2 = 2.5
        // Bug: using values.len() would give idx_end = 3, returning avg(3.0, 4.0) = 3.5
        // Fix: using sorted.len() gives idx_end = 2, returning avg(2.0, 3.0) = 2.5
        let span = Span::test_data();
        let values = vec![
            Value::test_float(f64::NAN),
            Value::test_float(f64::NAN),
            Value::test_float(1.0),
            Value::test_float(2.0),
            Value::test_float(3.0),
            Value::test_float(4.0),
        ];

        let result = median(&values, span, span).unwrap();
        assert_eq!(result, Value::test_float(2.5));
    }
}
