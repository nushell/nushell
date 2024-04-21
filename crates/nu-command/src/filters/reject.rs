use nu_engine::command_prelude::*;
use nu_protocol::ast::PathMember;
use std::{cmp::Reverse, collections::HashSet};

#[derive(Clone)]
pub struct Reject;

impl Command for Reject {
    fn name(&self) -> &str {
        "reject"
    }

    fn signature(&self) -> Signature {
        Signature::build("reject")
            .input_output_types(vec![
                (Type::record(), Type::record()),
                (Type::table(), Type::table()),
            ])
            .switch(
                "ignore-errors",
                "ignore missing data (make all cell path members optional)",
                Some('i'),
            )
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "The names of columns to remove from the table.",
            )
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Remove the given columns or rows from the table. Opposite of `select`."
    }

    fn extra_usage(&self) -> &str {
        "To remove a quantity of rows or columns, use `skip`, `drop`, or `drop column`."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["drop", "key"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let columns: Vec<Value> = call.rest(engine_state, stack, 0)?;
        let mut new_columns: Vec<CellPath> = vec![];
        for col_val in columns {
            let col_span = &col_val.span();
            match col_val {
                Value::CellPath { val, .. } => {
                    new_columns.push(val);
                }
                Value::String { val, .. } => {
                    let cv = CellPath {
                        members: vec![PathMember::String {
                            val: val.clone(),
                            span: *col_span,
                            optional: false,
                        }],
                    };
                    new_columns.push(cv.clone());
                }
                Value::Int { val, .. } => {
                    let cv = CellPath {
                        members: vec![PathMember::Int {
                            val: val as usize,
                            span: *col_span,
                            optional: false,
                        }],
                    };
                    new_columns.push(cv.clone());
                }
                x => {
                    return Err(ShellError::CantConvert {
                        to_type: "cell path".into(),
                        from_type: x.get_type().to_string(),
                        span: x.span(),
                        help: None,
                    });
                }
            }
        }
        let span = call.head;

        let ignore_errors = call.has_flag(engine_state, stack, "ignore-errors")?;
        if ignore_errors {
            for cell_path in &mut new_columns {
                cell_path.make_optional();
            }
        }

        reject(engine_state, span, input, new_columns)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Reject a column in the `ls` table",
                example: "ls | reject modified",
                result: None,
            },
            Example {
                description: "Reject a column in a table",
                example: "[[a, b]; [1, 2]] | reject a",
                result: Some(Value::test_list(
                    vec![Value::test_record(record! {
                        "b" => Value::test_int(2),
                    })],
                )),
            },
            Example {
                description: "Reject a row in a table",
                example: "[[a, b]; [1, 2] [3, 4]] | reject 1",
                result: Some(Value::test_list(
                    vec![Value::test_record(record! {
                        "a" =>  Value::test_int(1),
                        "b" =>  Value::test_int(2),
                    })],
                )),
            },
            Example {
                description: "Reject the specified field in a record",
                example: "{a: 1, b: 2} | reject a",
                result: Some(Value::test_record(record! {
                    "b" => Value::test_int(2),
                })),
            },
            Example {
                description: "Reject a nested field in a record",
                example: "{a: {b: 3, c: 5}} | reject a.b",
                result: Some(Value::test_record(record! {
                    "a" => Value::test_record(record! {
                        "c" => Value::test_int(5),
                    }),
                })),
            },
            Example {
                description: "Reject multiple rows",
                example: "[[name type size]; [Cargo.toml toml 1kb] [Cargo.lock toml 2kb] [file.json json 3kb]] | reject 0 2",
                result: None,
            },
            Example {
                description: "Reject multiple columns",
                example: "[[name type size]; [Cargo.toml toml 1kb] [Cargo.lock toml 2kb]] | reject type size",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! { "name" => Value::test_string("Cargo.toml") }),
                    Value::test_record(record! { "name" => Value::test_string("Cargo.lock") }),
                ])),
            },
            Example {
                description: "Reject multiple columns by spreading a list",
                example: "let cols = [type size]; [[name type size]; [Cargo.toml toml 1kb] [Cargo.lock toml 2kb]] | reject ...$cols",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! { "name" => Value::test_string("Cargo.toml") }),
                    Value::test_record(record! { "name" => Value::test_string("Cargo.lock") }),
                ])),
            },
        ]
    }
}

fn reject(
    _engine_state: &EngineState,
    span: Span,
    input: PipelineData,
    cell_paths: Vec<CellPath>,
) -> Result<PipelineData, ShellError> {
    let mut unique_rows: HashSet<usize> = HashSet::new();
    let metadata = input.metadata();
    let val = input.into_value(span);
    let mut val = val;
    let mut new_columns = vec![];
    let mut new_rows = vec![];
    for column in cell_paths {
        let CellPath { ref members } = column;
        match members.first() {
            Some(PathMember::Int { val, span, .. }) => {
                if members.len() > 1 {
                    return Err(ShellError::GenericError {
                        error: "Reject only allows row numbers for rows".into(),
                        msg: "extra after row number".into(),
                        span: Some(*span),
                        help: None,
                        inner: vec![],
                    });
                }
                if !unique_rows.contains(val) {
                    unique_rows.insert(*val);
                    new_rows.push(column);
                }
            }
            _ => {
                if !new_columns.contains(&column) {
                    new_columns.push(column)
                }
            }
        };
    }
    new_rows.sort_unstable_by_key(|k| {
        Reverse({
            match k.members[0] {
                PathMember::Int { val, .. } => val,
                PathMember::String { .. } => usize::MIN,
            }
        })
    });

    new_columns.append(&mut new_rows);
    for cell_path in new_columns {
        val.remove_data_at_cell_path(&cell_path.members)?;
    }
    Ok(val.into_pipeline_data_with_metadata(metadata))
}

#[cfg(test)]
mod test {
    #[test]
    fn test_examples() {
        use super::Reject;
        use crate::test_examples;
        test_examples(Reject {})
    }
}
