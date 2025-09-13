use super::hashable_value::HashableValue;
use itertools::Itertools;
use nu_engine::command_prelude::*;

use std::collections::HashMap;

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
            .input_output_types(vec![(Type::List(Box::new(Type::Any)), Type::table())])
            .optional(
                "column-name",
                SyntaxShape::String,
                "Column name to calc frequency, no need to provide if input is a list.",
            )
            .optional(
                "frequency-column-name",
                SyntaxShape::String,
                "Histogram's frequency column, default to be frequency column output.",
            )
            .param(
                Flag::new("percentage-type")
                    .short('t')
                    .arg(SyntaxShape::String)
                    .desc(
                        "percentage calculate method, can be 'normalize' or 'relative', in \
                         'normalize', defaults to be 'normalize'",
                    )
                    .completion(Completion::new_list(&["normalize", "relative"])),
            )
            .category(Category::Chart)
    }

    fn description(&self) -> &str {
        "Creates a new table with a histogram based on the column name passed in."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Compute a histogram of file types",
                example: "ls | histogram type",
                result: None,
            },
            Example {
                description: "Compute a histogram for the types of files, with frequency column \
                              named freq",
                example: "ls | histogram type freq",
                result: None,
            },
            Example {
                description: "Compute a histogram for a list of numbers",
                example: "[1 2 1] | histogram",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "value" =>      Value::test_int(1),
                        "count" =>      Value::test_int(2),
                        "quantile" =>   Value::test_float(0.6666666666666666),
                        "percentage" => Value::test_string("66.67%"),
                        "frequency" =>  Value::test_string("******************************************************************"),
                    }),
                    Value::test_record(record! {
                        "value" =>      Value::test_int(2),
                        "count" =>      Value::test_int(1),
                        "quantile" =>   Value::test_float(0.3333333333333333),
                        "percentage" => Value::test_string("33.33%"),
                        "frequency" =>  Value::test_string("*********************************"),
                    }),
                ])),
            },
            Example {
                description: "Compute a histogram for a list of numbers, and percentage is based \
                              on the maximum value",
                example: "[1 2 3 1 1 1 2 2 1 1] | histogram --percentage-type relative",
                result: None,
            },
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
                let forbidden_column_names = ["value", "count", "quantile", "percentage"];
                if forbidden_column_names.contains(&inner.item.as_str()) {
                    return Err(ShellError::TypeMismatch {
                        err_message: format!(
                            "frequency-column-name can't be {}",
                            forbidden_column_names
                                .iter()
                                .map(|val| format!("'{val}'"))
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                        span: inner.span,
                    });
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
                    return Err(ShellError::TypeMismatch {
                        err_message: "calc method can only be 'normalize' or 'relative'"
                            .to_string(),
                        span: inner.span,
                    });
                }
            },
        };

        let span = call.head;
        let data_as_value = input.into_value(span)?;
        let value_span = data_as_value.span();
        // `input` is not a list, here we can return an error.
        run_histogram(
            data_as_value.into_list()?,
            column_name,
            frequency_column_name,
            calc_method,
            span,
            // Note that as_list() filters out Value::Error here.
            value_span,
        )
    }
}

fn run_histogram(
    values: Vec<Value>,
    column_name: Option<Spanned<String>>,
    freq_column: String,
    calc_method: PercentageCalcMethod,
    head_span: Span,
    list_span: Span,
) -> Result<PipelineData, ShellError> {
    let mut inputs = vec![];
    // convert from inputs to hashable values.
    match column_name {
        None => {
            // some invalid input scenario needs to handle:
            // Expect input is a list of hashable value, if one value is not hashable, throw out error.
            for v in values {
                match v {
                    // Propagate existing errors.
                    Value::Error { error, .. } => return Err(*error),
                    _ => {
                        let t = v.get_type();
                        let span = v.span();
                        inputs.push(HashableValue::from_value(v, head_span).map_err(|_| {
                            ShellError::UnsupportedInput {
                                msg: "Since column-name was not provided, only lists of hashable \
                                      values are supported."
                                    .to_string(),
                                input: format!("input type: {t:?}"),
                                msg_span: head_span,
                                input_span: span,
                            }
                        })?)
                    }
                }
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
                    Value::Record { val, .. } => {
                        if let Some(v) = val.get(col_name)
                            && let Ok(v) = HashableValue::from_value(v.clone(), head_span)
                        {
                            inputs.push(v);
                        }
                    }
                    // Propagate existing errors.
                    Value::Error { error, .. } => return Err(*error),
                    _ => continue,
                }
            }

            if inputs.is_empty() {
                return Err(ShellError::CantFindColumn {
                    col_name: col_name.clone(),
                    span: Some(head_span),
                    src_span: list_span,
                });
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
    const MAX_FREQ_COUNT: f64 = 100.0;
    for (val, count) in counter.into_iter().sorted() {
        let quantile = match calc_method {
            PercentageCalcMethod::Normalize => count as f64 / total_cnt as f64,
            PercentageCalcMethod::Relative => count as f64 / max_cnt as f64,
        };

        let percentage = format!("{:.2}%", quantile * 100_f64);
        let freq = "*".repeat((MAX_FREQ_COUNT * quantile).floor() as usize);

        result.push((
            count, // attach count first for easily sorting.
            Value::record(
                record! {
                    value_column_name => val.into_value(),
                    "count" => Value::int(count, span),
                    "quantile" => Value::float(quantile, span),
                    "percentage" => Value::string(percentage, span),
                    freq_column => Value::string(freq, span),
                },
                span,
            ),
        ));
    }
    result.sort_by(|a, b| b.0.cmp(&a.0));
    Value::list(result.into_iter().map(|x| x.1).collect(), span).into_pipeline_data()
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
