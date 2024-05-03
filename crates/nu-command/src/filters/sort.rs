use alphanumeric_sort::compare_str;
use nu_engine::command_prelude::*;

use nu_utils::IgnoreCaseExt;
use std::cmp::Ordering;

#[derive(Clone)]
pub struct Sort;

impl Command for Sort {
    fn name(&self) -> &str {
        "sort"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("sort")
        .input_output_types(vec![(
            Type::List(Box::new(Type::Any)),
            Type::List(Box::new(Type::Any)),
        ), (Type::record(), Type::record()),])
    .switch("reverse", "Sort in reverse order", Some('r'))
            .switch(
                "ignore-case",
                "Sort string-based data case-insensitively",
                Some('i'),
            )
            .switch(
                "values",
                "If input is a single record, sort the record by values; ignored if input is not a single record",
                Some('v'),
            )
            .switch(
                "natural",
                "Sort alphanumeric string-based values naturally (1, 9, 10, 99, 100, ...)",
                Some('n'),
            )
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Sort in increasing order."
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "[2 0 1] | sort",
                description: "sort the list by increasing value",
                result: Some(Value::list(
                    vec![Value::test_int(0), Value::test_int(1), Value::test_int(2)],
                    Span::test_data(),
                )),
            },
            Example {
                example: "[2 0 1] | sort --reverse",
                description: "sort the list by decreasing value",
                result: Some(Value::list(
                    vec![Value::test_int(2), Value::test_int(1), Value::test_int(0)],
                    Span::test_data(),
                )),
            },
            Example {
                example: "[betty amy sarah] | sort",
                description: "sort a list of strings",
                result: Some(Value::list(
                    vec![
                        Value::test_string("amy"),
                        Value::test_string("betty"),
                        Value::test_string("sarah"),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                example: "[betty amy sarah] | sort --reverse",
                description: "sort a list of strings in reverse",
                result: Some(Value::list(
                    vec![
                        Value::test_string("sarah"),
                        Value::test_string("betty"),
                        Value::test_string("amy"),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Sort strings (case-insensitive)",
                example: "[airplane Truck Car] | sort -i",
                result: Some(Value::list(
                    vec![
                        Value::test_string("airplane"),
                        Value::test_string("Car"),
                        Value::test_string("Truck"),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Sort strings (reversed case-insensitive)",
                example: "[airplane Truck Car] | sort -i -r",
                result: Some(Value::list(
                    vec![
                        Value::test_string("Truck"),
                        Value::test_string("Car"),
                        Value::test_string("airplane"),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Sort record by key (case-insensitive)",
                example: "{b: 3, a: 4} | sort",
                result: Some(Value::test_record(record! {
                    "a" => Value::test_int(4),
                    "b" => Value::test_int(3),
                })),
            },
            Example {
                description: "Sort record by value",
                example: "{b: 4, a: 3, c:1} | sort -v",
                result: Some(Value::test_record(record! {
                    "c" => Value::test_int(1),
                    "a" => Value::test_int(3),
                    "b" => Value::test_int(4),
                })),
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
        let reverse = call.has_flag(engine_state, stack, "reverse")?;
        let insensitive = call.has_flag(engine_state, stack, "ignore-case")?;
        let natural = call.has_flag(engine_state, stack, "natural")?;
        let metadata = &input.metadata();

        let span = input.span().unwrap_or(call.head);
        match input {
            // Records have two sorting methods, toggled by presence or absence of -v
            PipelineData::Value(Value::Record { val, .. }, ..) => {
                let sort_by_value = call.has_flag(engine_state, stack, "values")?;
                let record = sort_record(
                    val.into_owned(),
                    span,
                    sort_by_value,
                    reverse,
                    insensitive,
                    natural,
                );
                Ok(record.into_pipeline_data())
            }
            // Other values are sorted here
            PipelineData::Value(v, ..)
                if !matches!(v, Value::List { .. } | Value::Range { .. }) =>
            {
                Ok(v.into_pipeline_data())
            }
            pipe_data => {
                let mut vec: Vec<_> = pipe_data.into_iter().collect();

                sort(&mut vec, call.head, insensitive, natural)?;

                if reverse {
                    vec.reverse()
                }

                let iter = vec.into_iter();
                match metadata {
                    Some(m) => Ok(iter
                        .into_pipeline_data_with_metadata(m.clone(), engine_state.ctrlc.clone())),
                    None => Ok(iter.into_pipeline_data(engine_state.ctrlc.clone())),
                }
            }
        }
    }
}

fn sort_record(
    record: Record,
    rec_span: Span,
    sort_by_value: bool,
    reverse: bool,
    insensitive: bool,
    natural: bool,
) -> Value {
    let mut input_pairs: Vec<(String, Value)> = record.into_iter().collect();
    input_pairs.sort_by(|a, b| {
        // Extract the data (if sort_by_value) or the column names for comparison
        let left_res = if sort_by_value {
            match &a.1 {
                Value::String { val, .. } => val.clone(),
                val => {
                    if let Ok(val) = val.coerce_string() {
                        val
                    } else {
                        // Values that can't be turned to strings are disregarded by the sort
                        // (same as in sort_utils.rs)
                        return Ordering::Equal;
                    }
                }
            }
        } else {
            a.0.clone()
        };
        let right_res = if sort_by_value {
            match &b.1 {
                Value::String { val, .. } => val.clone(),
                val => {
                    if let Ok(val) = val.coerce_string() {
                        val
                    } else {
                        // Values that can't be turned to strings are disregarded by the sort
                        // (same as in sort_utils.rs)
                        return Ordering::Equal;
                    }
                }
            }
        } else {
            b.0.clone()
        };

        // Fold case if case-insensitive
        let left = if insensitive {
            left_res.to_folded_case()
        } else {
            left_res
        };
        let right = if insensitive {
            right_res.to_folded_case()
        } else {
            right_res
        };

        if natural {
            compare_str(left, right)
        } else {
            left.cmp(&right)
        }
    });

    if reverse {
        input_pairs.reverse();
    }

    Value::record(input_pairs.into_iter().collect(), rec_span)
}

pub fn sort(
    vec: &mut [Value],
    span: Span,
    insensitive: bool,
    natural: bool,
) -> Result<(), ShellError> {
    match vec.first() {
        Some(Value::Record { val, .. }) => {
            let columns: Vec<String> = val.columns().cloned().collect();
            vec.sort_by(|a, b| process(a, b, &columns, span, insensitive, natural));
        }
        _ => {
            vec.sort_by(|a, b| {
                let span_a = a.span();
                let span_b = b.span();
                if insensitive {
                    let folded_left = match a {
                        Value::String { val, .. } => Value::string(val.to_folded_case(), span_a),
                        _ => a.clone(),
                    };

                    let folded_right = match b {
                        Value::String { val, .. } => Value::string(val.to_folded_case(), span_b),
                        _ => b.clone(),
                    };

                    if natural {
                        match (
                            folded_left.coerce_into_string(),
                            folded_right.coerce_into_string(),
                        ) {
                            (Ok(left), Ok(right)) => compare_str(left, right),
                            _ => Ordering::Equal,
                        }
                    } else {
                        folded_left
                            .partial_cmp(&folded_right)
                            .unwrap_or(Ordering::Equal)
                    }
                } else if natural {
                    match (a.coerce_str(), b.coerce_str()) {
                        (Ok(left), Ok(right)) => compare_str(left, right),
                        _ => Ordering::Equal,
                    }
                } else {
                    a.partial_cmp(b).unwrap_or(Ordering::Equal)
                }
            });
        }
    }
    Ok(())
}

pub fn process(
    left: &Value,
    right: &Value,
    columns: &[String],
    span: Span,
    insensitive: bool,
    natural: bool,
) -> Ordering {
    for column in columns {
        let left_value = left.get_data_by_key(column);

        let left_res = match left_value {
            Some(left_res) => left_res,
            None => Value::nothing(span),
        };

        let right_value = right.get_data_by_key(column);

        let right_res = match right_value {
            Some(right_res) => right_res,
            None => Value::nothing(span),
        };

        let result = if insensitive {
            let span_left = left_res.span();
            let span_right = right_res.span();
            let folded_left = match left_res {
                Value::String { val, .. } => Value::string(val.to_folded_case(), span_left),
                _ => left_res,
            };

            let folded_right = match right_res {
                Value::String { val, .. } => Value::string(val.to_folded_case(), span_right),
                _ => right_res,
            };
            if natural {
                match (
                    folded_left.coerce_into_string(),
                    folded_right.coerce_into_string(),
                ) {
                    (Ok(left), Ok(right)) => compare_str(left, right),
                    _ => Ordering::Equal,
                }
            } else {
                folded_left
                    .partial_cmp(&folded_right)
                    .unwrap_or(Ordering::Equal)
            }
        } else {
            left_res.partial_cmp(&right_res).unwrap_or(Ordering::Equal)
        };
        if result != Ordering::Equal {
            return result;
        }
    }

    Ordering::Equal
}

#[cfg(test)]
mod test {

    use nu_protocol::engine::CommandType;

    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Sort {})
    }

    #[test]
    fn test_command_type() {
        assert!(matches!(Sort.command_type(), CommandType::Builtin));
    }
}
