use chrono::{DateTime, FixedOffset};
use nu_engine::{column::column_does_not_exist, CallExt};
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Config, Example, IntoInterruptiblePipelineData, PipelineData, ShellError, Signature,
    Span, SyntaxShape, Value,
};
use std::cmp::Ordering;

#[derive(Clone)]
pub struct SortBy;

impl Command for SortBy {
    fn name(&self) -> &str {
        "sort_by"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("sort_by")
            .rest("columns", SyntaxShape::Any, "the column(s) to sort by")
            .switch("reverse", "Sort in reverse order", Some('r'))
            .switch(
                "insensitive",
                "Sort string-based columns case-insensitively",
                Some('i'),
            )
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Sort by the given columns, in increasing order."
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "[2 0 1] | sort_by",
                description: "sort the list by increasing value",
                result: Some(Value::List {
                    vals: vec![Value::test_int(0), Value::test_int(1), Value::test_int(2)],
                    span: Span::test_data(),
                }),
            },
            Example {
                example: "[2 0 1] | sort_by -r",
                description: "sort the list by decreasing value",
                result: Some(Value::List {
                    vals: vec![Value::test_int(2), Value::test_int(1), Value::test_int(0)],
                    span: Span::test_data(),
                }),
            },
            Example {
                example: "[betty amy sarah] | sort_by",
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
                example: "[betty amy sarah] | sort_by -r",
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
                example: "echo [airplane Truck Car] | sort_by -i",
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
                example: "echo [airplane Truck Car] | sort_by -i -r",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_string("Truck"),
                        Value::test_string("Car"),
                        Value::test_string("airplane"),
                    ],
                    span: Span::test_data(),
                }),
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
        let columns: Vec<String> = call.rest(engine_state, stack, 0)?;
        let reverse = call.has_flag("reverse");
        let insensitive = call.has_flag("insensitive");
        let metadata = &input.metadata();
        let config = stack.get_config()?;
        let mut vec: Vec<_> = input.into_iter().collect();

        sort(&mut vec, columns, call, insensitive, &config)?;

        if reverse {
            vec.reverse()
        }

        let iter = vec.into_iter();
        match &*metadata {
            Some(m) => {
                Ok(iter.into_pipeline_data_with_metadata(m.clone(), engine_state.ctrlc.clone()))
            }
            None => Ok(iter.into_pipeline_data(engine_state.ctrlc.clone())),
        }
    }
}

pub fn sort(
    vec: &mut [Value],
    columns: Vec<String>,
    call: &Call,
    insensitive: bool,
    config: &Config,
) -> Result<(), ShellError> {
    let should_sort_case_insensitively = insensitive
        && vec
            .iter()
            .all(|x| matches!(x.get_type(), nu_protocol::Type::String));

    match &vec[0] {
        Value::Record {
            cols,
            vals: _input_vals,
            ..
        } => {
            if columns.is_empty() {
                println!("sort_by requires a column name to sort table data");
                return Err(ShellError::CantFindColumn(call.head, call.head));
            }

            if column_does_not_exist(columns.clone(), cols.to_vec()) {
                return Err(ShellError::CantFindColumn(call.head, call.head));
            }

            vec.sort_by(|a, b| {
                process(
                    a,
                    b,
                    &columns[0],
                    call,
                    should_sort_case_insensitively,
                    config,
                )
                .expect("sort_by Value::Record bug")
                .compare()
            });
        }
        _ => {
            vec.sort_by(|a, b| {
                if should_sort_case_insensitively {
                    let lowercase_left = Value::string(
                        a.into_string("", config).to_ascii_lowercase(),
                        Span::test_data(),
                    );
                    let lowercase_right = Value::string(
                        b.into_string("", config).to_ascii_lowercase(),
                        Span::test_data(),
                    );
                    coerce_compare(&lowercase_left, &lowercase_right)
                        .expect("sort_by default bug")
                        .compare()
                } else {
                    coerce_compare(a, b).expect("sort_by default bug").compare()
                }
            });
        }
    }
    Ok(())
}

pub fn process(
    left: &Value,
    right: &Value,
    column: &str,
    call: &Call,
    insensitive: bool,
    config: &Config,
) -> Result<CompareValues, (&'static str, &'static str)> {
    let left_value = left.get_data_by_key(column);

    let left_res = match left_value {
        Some(left_res) => left_res,
        None => Value::Nothing { span: call.head },
    };

    let right_value = right.get_data_by_key(column);

    let right_res = match right_value {
        Some(right_res) => right_res,
        None => Value::Nothing { span: call.head },
    };

    if insensitive {
        let lowercase_left = Value::string(
            left_res.into_string("", config).to_ascii_lowercase(),
            Span::test_data(),
        );
        let lowercase_right = Value::string(
            right_res.into_string("", config).to_ascii_lowercase(),
            Span::test_data(),
        );
        coerce_compare(&lowercase_left, &lowercase_right)
    } else {
        coerce_compare(&left_res, &right_res)
    }
}

#[derive(Debug)]
pub enum CompareValues {
    Ints(i64, i64),
    Floats(f64, f64),
    String(String, String),
    Booleans(bool, bool),
    Filesize(i64, i64),
    Date(DateTime<FixedOffset>, DateTime<FixedOffset>),
}

impl CompareValues {
    pub fn compare(&self) -> std::cmp::Ordering {
        match self {
            CompareValues::Ints(left, right) => left.cmp(right),
            CompareValues::Floats(left, right) => process_floats(left, right),
            CompareValues::String(left, right) => left.cmp(right),
            CompareValues::Booleans(left, right) => left.cmp(right),
            CompareValues::Filesize(left, right) => left.cmp(right),
            CompareValues::Date(left, right) => left.cmp(right),
        }
    }
}

pub fn process_floats(left: &f64, right: &f64) -> std::cmp::Ordering {
    let result = left.partial_cmp(right);
    match result {
        Some(Ordering::Greater) => Ordering::Greater,
        Some(Ordering::Less) => Ordering::Less,
        _ => Ordering::Equal,
    }
}

pub fn coerce_compare(
    left: &Value,
    right: &Value,
) -> Result<CompareValues, (&'static str, &'static str)> {
    match (left, right) {
        (Value::Float { val: left, .. }, Value::Float { val: right, .. }) => {
            Ok(CompareValues::Floats(*left, *right))
        }
        (Value::Filesize { val: left, .. }, Value::Filesize { val: right, .. }) => {
            Ok(CompareValues::Filesize(*left, *right))
        }
        (Value::Date { val: left, .. }, Value::Date { val: right, .. }) => {
            Ok(CompareValues::Date(*left, *right))
        }
        (Value::Int { val: left, .. }, Value::Int { val: right, .. }) => {
            Ok(CompareValues::Ints(*left, *right))
        }
        (Value::String { val: left, .. }, Value::String { val: right, .. }) => {
            Ok(CompareValues::String(left.clone(), right.clone()))
        }
        (Value::Bool { val: left, .. }, Value::Bool { val: right, .. }) => {
            Ok(CompareValues::Booleans(*left, *right))
        }
        _ => Err(("coerce_compare_left", "coerce_compare_right")),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SortBy {})
    }
}
