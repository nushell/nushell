use std::cmp::Ordering;

use crate::math::avg::average;
use crate::math::utils::run_with_function;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, Span, Type, Value};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "math median"
    }

    fn signature(&self) -> Signature {
        Signature::build("math median")
            .input_output_types(vec![
                (Type::List(Box::new(Type::Number)), Type::Number),
                (Type::Table(vec![]), Type::Record(vec![])),
            ])
            .category(Category::Math)
    }

    fn usage(&self) -> &str {
        "Computes the median of a list of numbers"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["middle", "statistics"]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        run_with_function(call, input, median)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Compute the median of a list of numbers",
                example: "[3 8 9 12 12 15] | math median",
                result: Some(Value::Float {
                    val: 10.5,
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Compute the medians of the columns of a table",
                example: "[{a: 1 b: 3} {a: 2 b: -1} {a: -3 b: 5}] | math median",
                result: Some(Value::Record {
                    cols: vec!["a".to_string(), "b".to_string()],
                    vals: vec![Value::test_int(1), Value::test_int(3)],
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

enum Pick {
    MedianAverage,
    Median,
}

pub fn median(values: &[Value], head: &Span) -> Result<Value, ShellError> {
    let take = if values.len() % 2 == 0 {
        Pick::MedianAverage
    } else {
        Pick::Median
    };

    let mut sorted = vec![];

    for item in values {
        sorted.push(item.clone());
    }

    if let Some(Err(values)) = values
        .windows(2)
        .map(|elem| {
            if elem[0].partial_cmp(&elem[1]).is_none() {
                return Err(ShellError::OperatorMismatch {
                    op_span: *head,
                    lhs_ty: elem[0].get_type(),
                    lhs_span: elem[0].span()?,
                    rhs_ty: elem[1].get_type(),
                    rhs_span: elem[1].span()?,
                });
            }
            Ok(elem[0].partial_cmp(&elem[1]).unwrap_or(Ordering::Equal))
        })
        .find(|elem| elem.is_err())
    {
        return Err(values);
    }

    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));

    match take {
        Pick::Median => {
            let idx = (values.len() as f64 / 2.0).floor() as usize;
            let out = sorted
                .get(idx)
                .ok_or_else(|| ShellError::UnsupportedInput("Empty input".to_string(), *head))?;
            Ok(out.clone())
        }
        Pick::MedianAverage => {
            let idx_end = values.len() / 2;
            let idx_start = idx_end - 1;

            let left = sorted
                .get(idx_start)
                .ok_or_else(|| ShellError::UnsupportedInput("Empty input".to_string(), *head))?
                .clone();

            let right = sorted
                .get(idx_end)
                .ok_or_else(|| ShellError::UnsupportedInput("Empty input".to_string(), *head))?
                .clone();

            average(&[left, right], head)
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
