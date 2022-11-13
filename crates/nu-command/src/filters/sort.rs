use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData, ShellError,
    Signature, Span, Type, Value,
};
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
        ), (Type::Record(vec![]), Type::Record(vec![])),])
    .switch("reverse", "Sort in reverse order", Some('r'))
            .switch(
                "insensitive",
                "Sort string-based columns case-insensitively",
                Some('i'),
            )
            .switch(
                "values",
                "If input is a single record, sort the record by values, ignored if input is not a single record",
                Some('v'),
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
                result: Some(Value::List {
                    vals: vec![Value::test_int(0), Value::test_int(1), Value::test_int(2)],
                    span: Span::test_data(),
                }),
            },
            Example {
                example: "[2 0 1] | sort -r",
                description: "sort the list by decreasing value",
                result: Some(Value::List {
                    vals: vec![Value::test_int(2), Value::test_int(1), Value::test_int(0)],
                    span: Span::test_data(),
                }),
            },
            Example {
                example: "[betty amy sarah] | sort",
                description: "sort a list of strings",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_string("amy"),
                        Value::test_string("betty"),
                        Value::test_string("sarah"),
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                example: "[betty amy sarah] | sort -r",
                description: "sort a list of strings in reverse",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_string("sarah"),
                        Value::test_string("betty"),
                        Value::test_string("amy"),
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Sort strings (case-insensitive)",
                example: "echo [airplane Truck Car] | sort -i",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_string("airplane"),
                        Value::test_string("Car"),
                        Value::test_string("Truck"),
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Sort strings (reversed case-insensitive)",
                example: "echo [airplane Truck Car] | sort -i -r",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_string("Truck"),
                        Value::test_string("Car"),
                        Value::test_string("airplane"),
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Sort record by key",
                example: "{b: 3, a: 4} | sort",
                result: Some(Value::Record {
                    cols: vec!["a".to_string(), "b".to_string()],
                    vals: vec![Value::test_int(4), Value::test_int(3)],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Sort record by value",
                example: "{a: 4, b: 3} | sort",
                result: Some(Value::Record {
                    cols: vec!["b".to_string(), "a".to_string()],
                    vals: vec![Value::test_int(3), Value::test_int(4)],
                    span: Span::test_data(),
                }),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let reverse = call.has_flag("reverse");
        let insensitive = call.has_flag("insensitive");
        let metadata = &input.metadata();

        match input {
            PipelineData::Value(Value::Record { cols, vals, span }, ..) => {
                let sort_by_value = call.has_flag("values");
                let record = sort_record(cols, vals, span, sort_by_value);
                Ok(record.into_pipeline_data())
            }
            PipelineData::Value(v, ..)
                if !matches!(v, Value::List { .. } | Value::Range { .. }) =>
            {
                Ok(v.into_pipeline_data())
            }
            pipe_data => {
                let mut vec: Vec<_> = pipe_data.into_iter().collect();

                sort(&mut vec, call.head, insensitive)?;

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

fn sort_record(cols: Vec<String>, vals: Vec<Value>, rec_span: Span, sort_by_value: bool) -> Value {
    let mut input_pairs: Vec<(String, Value)> = cols.into_iter().zip(vals).collect();
    if sort_by_value {
        input_pairs.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));
    } else {
        input_pairs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));
    }
    let mut new_cols = Vec::with_capacity(input_pairs.len());
    let mut new_vals = Vec::with_capacity(input_pairs.len());
    for (col, val) in input_pairs {
        new_cols.push(col);
        new_vals.push(val)
    }
    Value::Record {
        cols: new_cols,
        vals: new_vals,
        span: rec_span,
    }
}

pub fn sort(vec: &mut [Value], span: Span, insensitive: bool) -> Result<(), ShellError> {
    if vec.is_empty() {
        return Err(ShellError::GenericError(
            "no values to work with".to_string(),
            "".to_string(),
            None,
            Some("no values to work with".to_string()),
            Vec::new(),
        ));
    }

    match &vec[0] {
        Value::Record {
            cols,
            vals: _input_vals,
            ..
        } => {
            let columns = cols.clone();
            vec.sort_by(|a, b| process(a, b, &columns, span, insensitive));
        }
        _ => {
            vec.sort_by(|a, b| {
                if insensitive {
                    let lowercase_left = match a {
                        Value::String { val, span } => Value::String {
                            val: val.to_ascii_lowercase(),
                            span: *span,
                        },
                        _ => a.clone(),
                    };

                    let lowercase_right = match b {
                        Value::String { val, span } => Value::String {
                            val: val.to_ascii_lowercase(),
                            span: *span,
                        },
                        _ => b.clone(),
                    };

                    lowercase_left
                        .partial_cmp(&lowercase_right)
                        .unwrap_or(Ordering::Equal)
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
) -> Ordering {
    for column in columns {
        let left_value = left.get_data_by_key(column);

        let left_res = match left_value {
            Some(left_res) => left_res,
            None => Value::Nothing { span },
        };

        let right_value = right.get_data_by_key(column);

        let right_res = match right_value {
            Some(right_res) => right_res,
            None => Value::Nothing { span },
        };

        let result = if insensitive {
            let lowercase_left = match left_res {
                Value::String { val, span } => Value::String {
                    val: val.to_ascii_lowercase(),
                    span,
                },
                _ => left_res,
            };

            let lowercase_right = match right_res {
                Value::String { val, span } => Value::String {
                    val: val.to_ascii_lowercase(),
                    span,
                },
                _ => right_res,
            };
            lowercase_left
                .partial_cmp(&lowercase_right)
                .unwrap_or(Ordering::Equal)
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
