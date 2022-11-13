use super::hashable_value::HashableValue;
use itertools::Itertools;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape,
    Type, Value,
};
use std::collections::HashMap;
use std::iter;

#[derive(Clone)]
pub struct Histogram;

enum PercentageCalcMethod {
    Normalize,
    Relative,
}

impl Command for Histogram {
    fn name(&self) -> &str {
        "histogram"
    }

    fn signature(&self) -> Signature {
        Signature::build("histogram")
            .input_output_types(vec![(Type::List(Box::new(Type::Any)), Type::Table(vec![])),])
            .optional("column-name", SyntaxShape::String, "column name to calc frequency, no need to provide if input is just a list")
            .optional("frequency-column-name", SyntaxShape::String, "histogram's frequency column, default to be frequency column output")
            .named("percentage-type", SyntaxShape::String, "percentage calculate method, can be 'normalize' or 'relative', in 'normalize', defaults to be 'normalize'", Some('t'))
    }

    fn usage(&self) -> &str {
        "Creates a new table with a histogram based on the column name passed in."
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Compute a histogram of file types",
                example: "ls | histogram type",
                result: None,
            },
            Example {
                description:
                    "Compute a histogram for the types of files, with frequency column named freq",
                example: "ls | histogram type freq",
                result: None,
            },
            Example {
                description: "Compute a histogram for a list of numbers",
                example: "echo [1 2 1] | histogram",
                result: Some(Value::List {
                        vals: vec![Value::Record {
                            cols: vec!["value".to_string(), "count".to_string(), "quantile".to_string(), "percentage".to_string(), "frequency".to_string()],
                            vals: vec![
                                Value::test_int(1),
                                Value::test_int(2),
                                Value::test_float(0.6666666666666666),
                                Value::test_string("66.67%"),
                                Value::test_string("******************************************************************"),
                            ],
                            span: Span::test_data(),
                        },
                        Value::Record {
                            cols: vec!["value".to_string(), "count".to_string(), "quantile".to_string(), "percentage".to_string(), "frequency".to_string()],
                            vals: vec![
                                Value::test_int(2),
                                Value::test_int(1),
                                Value::test_float(0.3333333333333333),
                                Value::test_string("33.33%"),
                                Value::test_string("*********************************"),
                            ],
                            span: Span::test_data(),
                        }],
                        span: Span::test_data(),
                    }
                 ),
            },
            Example {
                description: "Compute a histogram for a list of numbers, and percentage is based on the maximum value",
                example: "echo [1 2 3 1 1 1 2 2 1 1] | histogram --percentage-type relative",
                result: None,
            }
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // input check.
        let column_name: Option<Spanned<String>> = call.opt(engine_state, stack, 0)?;
        let frequency_name_arg = call.opt::<Spanned<String>>(engine_state, stack, 1)?;
        let frequency_column_name = match frequency_name_arg {
            Some(inner) => {
                let span = inner.span;
                if ["value", "count", "quantile", "percentage"].contains(&inner.item.as_str()) {
                    return Err(ShellError::UnsupportedInput(
                        "frequency-column-name can't be 'value', 'count' or 'percentage'"
                            .to_string(),
                        span,
                    ));
                }
                inner.item
            }
            None => "frequency".to_string(),
        };

        let calc_method: Option<Spanned<String>> =
            call.get_flag(engine_state, stack, "percentage-type")?;
        let calc_method = match calc_method {
            None => PercentageCalcMethod::Normalize,
            Some(inner) => match inner.item.as_str() {
                "normalize" => PercentageCalcMethod::Normalize,
                "relative" => PercentageCalcMethod::Relative,
                _ => {
                    return Err(ShellError::UnsupportedInput(
                        "calc method can only be 'normalize' or 'relative'".to_string(),
                        inner.span,
                    ))
                }
            },
        };

        let span = call.head;
        let data_as_value = input.into_value(span);
        // `input` is not a list, here we can return an error.
        match data_as_value.as_list() {
            Ok(list_value) => run_histogram(
                list_value.to_vec(),
                column_name,
                frequency_column_name,
                calc_method,
                span,
            ),
            Err(e) => Err(e),
        }
    }
}

fn run_histogram(
    values: Vec<Value>,
    column_name: Option<Spanned<String>>,
    freq_column: String,
    calc_method: PercentageCalcMethod,
    head_span: Span,
) -> Result<PipelineData, ShellError> {
    let mut inputs = vec![];
    // convert from inputs to hashable values.
    match column_name {
        None => {
            // some invalid input scenario needs to handle:
            // Expect input is a list of hashable value, if one value is not hashable, throw out error.
            for v in values {
                let current_span = v.span().unwrap_or(head_span);
                inputs.push(HashableValue::from_value(v, head_span).map_err(|_| {
                    ShellError::UnsupportedInput(
                        "--column-name is not provided, can only support a list of simple value."
                            .to_string(),
                        current_span,
                    )
                })?);
            }
        }
        Some(ref col) => {
            // some invalid input scenario needs to handle:
            // * item in `input` is not a record, just skip it.
            // * a record doesn't contain specific column, just skip it.
            // * all records don't contain specific column, throw out error, indicate at least one row should contains specific column.
            // * a record contain a value which can't be hashed, skip it.
            let col_name = &col.item;
            for v in values {
                match v {
                    // parse record, and fill valid value to actual input.
                    Value::Record { cols, vals, .. } => {
                        for (c, v) in iter::zip(cols, vals) {
                            if &c == col_name {
                                if let Ok(v) = HashableValue::from_value(v, head_span) {
                                    inputs.push(v);
                                }
                            }
                        }
                    }
                    _ => continue,
                }
            }

            if inputs.is_empty() {
                return Err(ShellError::UnsupportedInput(
                    format!("expect input is table, and inputs doesn't contain any value which has {col_name} column"),
                    head_span,
                ));
            }
        }
    }

    let value_column_name = column_name
        .map(|x| x.item)
        .unwrap_or_else(|| "value".to_string());
    Ok(histogram_impl(
        inputs,
        &value_column_name,
        calc_method,
        &freq_column,
        head_span,
    ))
}

fn histogram_impl(
    inputs: Vec<HashableValue>,
    value_column_name: &str,
    calc_method: PercentageCalcMethod,
    freq_column: &str,
    span: Span,
) -> PipelineData {
    // here we can make sure that inputs is not empty, and every elements
    // is a simple val and ok to make count.
    let mut counter = HashMap::new();
    let mut max_cnt = 0;
    let total_cnt = inputs.len();
    for i in inputs {
        let new_cnt = *counter.get(&i).unwrap_or(&0) + 1;
        counter.insert(i, new_cnt);
        if new_cnt > max_cnt {
            max_cnt = new_cnt;
        }
    }

    let mut result = vec![];
    let result_cols = vec![
        value_column_name.to_string(),
        "count".to_string(),
        "quantile".to_string(),
        "percentage".to_string(),
        freq_column.to_string(),
    ];
    const MAX_FREQ_COUNT: f64 = 100.0;
    for (val, count) in counter.into_iter().sorted() {
        let quantile = match calc_method {
            PercentageCalcMethod::Normalize => count as f64 / total_cnt as f64,
            PercentageCalcMethod::Relative => count as f64 / max_cnt as f64,
        };

        let percentage = format!("{:.2}%", quantile * 100_f64);
        let freq = "*".repeat((MAX_FREQ_COUNT * quantile).floor() as usize);

        result.push(Value::Record {
            cols: result_cols.clone(),
            vals: vec![
                val.into_value(),
                Value::Int { val: count, span },
                Value::Float {
                    val: quantile,
                    span,
                },
                Value::String {
                    val: percentage,
                    span,
                },
                Value::String { val: freq, span },
            ],
            span,
        });
    }
    Value::List { vals: result, span }.into_pipeline_data()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Histogram)
    }
}
