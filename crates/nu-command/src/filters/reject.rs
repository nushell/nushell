use nu_engine::CallExt;
use nu_protocol::ast::{Call, CellPath, PathMember};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, Record, ShellError, Signature, Span,
    SyntaxShape, Type, Value,
};

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
                SyntaxShape::OneOf(vec![
                    SyntaxShape::CellPath,
                    SyntaxShape::List(Box::new(SyntaxShape::CellPath)),
                ]),
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
        let columns: Vec<Value> = call.rest(engine_state, stack, 0)?;
        let mut new_columns: Vec<CellPath> = vec![];
        for col_val in columns {
            let col_span = &col_val.span();
            match col_val {
                Value::CellPath { val, .. } => {
                    new_columns.push(val);
                }
                Value::List { vals, .. } => {
                    for value in vals {
                        let val_span = &value.span();
                        match value {
                            Value::String { val, .. } => {
                                let cv = CellPath {
                                    members: vec![PathMember::String {
                                        val: val.clone(),
                                        span: *val_span,
                                        optional: false,
                                    }],
                                };
                                new_columns.push(cv.clone());
                            }
                            Value::Int { val, .. } => {
                                let cv = CellPath {
                                    members: vec![PathMember::Int {
                                        val: val as usize,
                                        span: *val_span,
                                        optional: false,
                                    }],
                                };
                                new_columns.push(cv.clone());
                            }
                            y => {
                                return Err(ShellError::CantConvert {
                                    to_type: "cell path".into(),
                                    from_type: y.get_type().to_string(),
                                    span: y.span(),
                                    help: None,
                                });
                            }
                        }
                    }
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
                result: Some(Value::List {
                    vals: vec![Value::test_record(Record {
                        cols: vec!["b".to_string()],
                        vals: vec![Value::test_int(2)],
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
    let val = input.into_value(span);
    let mut val = val;
    let mut columns = vec![];
    for c in cell_paths {
        if !columns.contains(&c) {
            columns.push(c);
        }
    }
    for cell_path in columns {
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
