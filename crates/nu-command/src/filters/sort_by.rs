use chrono::{DateTime, FixedOffset};
use nu_engine::column::column_does_not_exist;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, PipelineData, ShellError, Signature, Span,
    SyntaxShape, Value,
};

#[derive(Clone)]
pub struct SortBy;

impl Command for SortBy {
    fn name(&self) -> &str {
        "sort-by"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("sort-by")
            .rest("columns", SyntaxShape::Any, "the column(s) to sort by")
            .switch("reverse", "Sort in reverse order", Some('r'))
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Sort by the given columns, in increasing order."
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "[2 0 1] | sort-by",
                description: "sort the list by increasing value",
                result: Some(Value::List {
                    vals: vec![Value::test_int(0), Value::test_int(1), Value::test_int(2)],
                    span: Span::test_data(),
                }),
            },
            Example {
                example: "[2 0 1] | sort-by -r",
                description: "sort the list by decreasing value",
                result: Some(Value::List {
                    vals: vec![Value::test_int(2), Value::test_int(1), Value::test_int(0)],
                    span: Span::test_data(),
                }),
            },
            Example {
                example: "[betty amy sarah] | sort-by",
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
                example: "[betty amy sarah] | sort-by -r",
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
        let metadata = &input.metadata();
        let mut vec: Vec<_> = input.into_iter().collect();

        sort(&mut vec, columns, call)?;

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

pub fn sort(vec: &mut [Value], columns: Vec<String>, call: &Call) -> Result<(), ShellError> {
    match &vec[0] {
        Value::Record {
            cols,
            vals: _input_vals,
            ..
        } => {
            if columns.is_empty() {
                println!("sort-by requires a column name to sort table data");
                return Err(ShellError::CantFindColumn(call.head, call.head));
            }

            if column_does_not_exist(columns.clone(), cols.to_vec()) {
                return Err(ShellError::CantFindColumn(call.head, call.head));
            }

            vec.sort_by(|a, b| {
                process(a, b, &columns[0], call)
                    .expect("sort_by Value::Record bug")
                    .compare()
            });
        }
        _ => {
            vec.sort_by(|a, b| coerce_compare(a, b).expect("sort_by default bug").compare());
        }
    }
    Ok(())
}

pub fn process(
    left: &Value,
    right: &Value,
    column: &str,
    call: &Call,
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

    coerce_compare(&left_res, &right_res)
}

#[derive(Debug)]
pub enum CompareValues {
    Ints(i64, i64),
    // Floats(f64, f64),
    String(String, String),
    Booleans(bool, bool),
    Filesize(i64, i64),
    Date(DateTime<FixedOffset>, DateTime<FixedOffset>),
}

impl CompareValues {
    pub fn compare(&self) -> std::cmp::Ordering {
        match self {
            CompareValues::Ints(left, right) => left.cmp(right),
            // f64: std::cmp::Ord is required
            // CompareValues::Floats(left, right) => left.cmp(right),
            CompareValues::String(left, right) => left.cmp(right),
            CompareValues::Booleans(left, right) => left.cmp(right),
            CompareValues::Filesize(left, right) => left.cmp(right),
            CompareValues::Date(left, right) => left.cmp(right),
        }
    }
}

pub fn coerce_compare(
    left: &Value,
    right: &Value,
) -> Result<CompareValues, (&'static str, &'static str)> {
    match (left, right) {
        // (Value::Float { val: left, .. }, Value::Float { val: right, .. }) => {
        //     Ok(CompareValues::Floats(*left, *right))
        // }
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
