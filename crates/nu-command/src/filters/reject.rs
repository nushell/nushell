use nu_engine::CallExt;
use nu_protocol::ast::{Call, CellPath, PathMember};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, Record, ShellError, Signature, Span,
    SyntaxShape, Type, Value,
};
use std::cmp::Reverse;
use std::collections::HashSet;

#[derive(Clone)]
pub struct Reject;

impl Command for Reject {
    fn name(&self) -> &str {
        "reject"
    }

    fn signature(&self) -> Signature {
        Signature::build("reject")
            .input_output_types(vec![
                (Type::Record(vec![]), Type::Record(vec![])),
                (Type::Table(vec![]), Type::Table(vec![])),
            ])
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "the names of columns to remove from the table",
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
        let columns: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
        let span = call.head;
        reject(engine_state, span, input, columns)
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
                result: Some(Value::List {
                    vals: vec![Value::test_record(Record {
                        cols: vec!["b".to_string()],
                        vals: vec![Value::test_int(2)],
                    })],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Reject a row in a table",
                example: "[[a, b]; [1, 2] [3, 4]] | reject 1",
                result: Some(Value::List {
                    vals: vec![Value::test_record(Record {
                        cols: vec!["a".to_string(), "b".to_string()],
                        vals: vec![Value::test_int(1), Value::test_int(2)],
                    })],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Reject the specified field in a record",
                example: "{a: 1, b: 2} | reject a",
                result: Some(Value::test_record(Record {
                    cols: vec!["b".into()],
                    vals: vec![Value::test_int(2)],
                })),
            },
            Example {
                description: "Reject a nested field in a record",
                example: "{a: {b: 3, c: 5}} | reject a.b",
                result: Some(Value::test_record(Record {
                    cols: vec!["a".into()],
                    vals: vec![Value::test_record(Record {
                        cols: vec!["c".into()],
                        vals: vec![Value::test_int(5)],
                    })],
                })),
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
    let val = input.into_value(span);
    let mut val = val;
    let mut new_columns = vec![];
    let mut new_rows = vec![];
    for column in cell_paths {
        let CellPath { ref members } = column;
        match members.get(0) {
            Some(PathMember::Int { val, span, .. }) => {
                if members.len() > 1 {
                    return Err(ShellError::GenericError(
                        "Reject only allows row numbers for rows".into(),
                        "extra after row number".into(),
                        Some(*span),
                        None,
                        Vec::new(),
                    ));
                }
                if unique_rows.contains(val) {
                    return Err(ShellError::GenericError(
                        "Reject can't get the same row twice".into(),
                        "duplicated row index".into(),
                        Some(*span),
                        None,
                        Vec::new(),
                    ));
                }
                unique_rows.insert(*val);
                new_rows.push(column);
            }
            _ => new_columns.push(column),
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
    Ok(val.into_pipeline_data())
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
