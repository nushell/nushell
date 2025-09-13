use std::ops::Not;

use nu_engine::command_prelude::*;

#[derive(Clone, Debug)]
enum Location {
    Before(Spanned<String>),
    After(Spanned<String>),
    Last,
    First,
}

#[derive(Clone)]
pub struct Move;

impl Command for Move {
    fn name(&self) -> &str {
        "move"
    }

    fn description(&self) -> &str {
        "Moves columns relative to other columns or make them the first/last columns. Flags are mutually exclusive."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("move")
            .input_output_types(vec![
                (Type::record(), Type::record()),
                (Type::table(), Type::table()),
            ])
            .rest("columns", SyntaxShape::String, "The columns to move.")
            .named(
                "after",
                SyntaxShape::String,
                "the column that will precede the columns moved",
                None,
            )
            .named(
                "before",
                SyntaxShape::String,
                "the column that will be the next after the columns moved",
                None,
            )
            .switch("first", "makes the columns be the first ones", None)
            .switch("last", "makes the columns be the last ones", None)
            .category(Category::Filters)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: "[[name value index]; [foo a 1] [bar b 2] [baz c 3]] | move index --before name",
                description: "Move a column before the first column",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "index" => Value::test_int(1),
                        "name" =>  Value::test_string("foo"),
                        "value" => Value::test_string("a"),
                    }),
                    Value::test_record(record! {
                        "index" => Value::test_int(2),
                        "name" =>  Value::test_string("bar"),
                        "value" => Value::test_string("b"),
                    }),
                    Value::test_record(record! {
                        "index" => Value::test_int(3),
                        "name" =>  Value::test_string("baz"),
                        "value" => Value::test_string("c"),
                    }),
                ])),
            },
            Example {
                example: "[[name value index]; [foo a 1] [bar b 2] [baz c 3]] | move value name --after index",
                description: "Move multiple columns after the last column and reorder them",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "index" => Value::test_int(1),
                        "value" => Value::test_string("a"),
                        "name" =>  Value::test_string("foo"),
                    }),
                    Value::test_record(record! {
                        "index" => Value::test_int(2),
                        "value" => Value::test_string("b"),
                        "name" =>  Value::test_string("bar"),
                    }),
                    Value::test_record(record! {
                        "index" => Value::test_int(3),
                        "value" => Value::test_string("c"),
                        "name" =>  Value::test_string("baz"),
                    }),
                ])),
            },
            Example {
                example: "{ name: foo, value: a, index: 1 } | move name --before index",
                description: "Move columns of a record",
                result: Some(Value::test_record(record! {
                    "value" => Value::test_string("a"),
                    "name" => Value::test_string("foo"),
                    "index" => Value::test_int(1),
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
        let head = call.head;
        let columns: Vec<Value> = call.rest(engine_state, stack, 0)?;
        let after: Option<Value> = call.get_flag(engine_state, stack, "after")?;
        let before: Option<Value> = call.get_flag(engine_state, stack, "before")?;
        let first = call.has_flag(engine_state, stack, "first")?;
        let last = call.has_flag(engine_state, stack, "last")?;

        let location = match (after, before, first, last) {
            (Some(v), None, false, false) => Location::After(Spanned {
                span: v.span(),
                item: v.coerce_into_string()?,
            }),
            (None, Some(v), false, false) => Location::Before(Spanned {
                span: v.span(),
                item: v.coerce_into_string()?,
            }),
            (None, None, true, false) => Location::First,
            (None, None, false, true) => Location::Last,
            (None, None, false, false) => {
                return Err(ShellError::GenericError {
                    error: "Cannot move columns".into(),
                    msg: "Missing required location flag".into(),
                    span: Some(head),
                    help: None,
                    inner: vec![],
                });
            }
            _ => {
                return Err(ShellError::GenericError {
                    error: "Cannot move columns".into(),
                    msg: "Use only a single flag".into(),
                    span: Some(head),
                    help: None,
                    inner: vec![],
                });
            }
        };

        let metadata = input.metadata();

        match input {
            PipelineData::Value(Value::List { .. }, ..) | PipelineData::ListStream { .. } => {
                let res = input.into_iter().map(move |x| match x.as_record() {
                    Ok(record) => match move_record_columns(record, &columns, &location, head) {
                        Ok(val) => val,
                        Err(error) => Value::error(error, head),
                    },
                    Err(error) => Value::error(error, head),
                });

                Ok(res.into_pipeline_data_with_metadata(
                    head,
                    engine_state.signals().clone(),
                    metadata,
                ))
            }
            PipelineData::Value(Value::Record { val, .. }, ..) => {
                Ok(move_record_columns(&val, &columns, &location, head)?.into_pipeline_data())
            }
            other => Err(ShellError::OnlySupportsThisInputType {
                exp_input_type: "record or table".to_string(),
                wrong_type: other.get_type().to_string(),
                dst_span: head,
                src_span: Span::new(head.start, head.start),
            }),
        }
    }
}

// Move columns within a record
fn move_record_columns(
    record: &Record,
    columns: &[Value],
    location: &Location,
    span: Span,
) -> Result<Value, ShellError> {
    let mut column_idx: Vec<usize> = Vec::with_capacity(columns.len());

    // Find indices of columns to be moved
    for column in columns.iter() {
        if let Some(idx) = record.index_of(column.coerce_string()?) {
            column_idx.push(idx);
        } else {
            return Err(ShellError::GenericError {
                error: "Cannot move columns".into(),
                msg: "column does not exist".into(),
                span: Some(column.span()),
                help: None,
                inner: vec![],
            });
        }
    }

    let mut out = Record::with_capacity(record.len());

    match &location {
        Location::Before(pivot) | Location::After(pivot) => {
            // Check if pivot exists
            if !record.contains(&pivot.item) {
                return Err(ShellError::GenericError {
                    error: "Cannot move columns".into(),
                    msg: "column does not exist".into(),
                    span: Some(pivot.span),
                    help: None,
                    inner: vec![],
                });
            }

            for (i, (inp_col, inp_val)) in record.iter().enumerate() {
                if inp_col == &pivot.item {
                    // Check if this pivot is also a column we are supposed to move
                    if column_idx.contains(&i) {
                        return Err(ShellError::IncompatibleParameters {
                            left_message: "Column cannot be moved".to_string(),
                            left_span: inp_val.span(),
                            right_message: "relative to itself".to_string(),
                            right_span: pivot.span,
                        });
                    }

                    if let Location::After(..) = location {
                        out.push(inp_col.clone(), inp_val.clone());
                    }

                    insert_moved(record, span, &column_idx, &mut out)?;

                    if let Location::Before(..) = location {
                        out.push(inp_col.clone(), inp_val.clone());
                    }
                } else if !column_idx.contains(&i) {
                    out.push(inp_col.clone(), inp_val.clone());
                }
            }
        }
        Location::First => {
            insert_moved(record, span, &column_idx, &mut out)?;

            out.extend(where_unmoved(record, &column_idx));
        }
        Location::Last => {
            out.extend(where_unmoved(record, &column_idx));

            insert_moved(record, span, &column_idx, &mut out)?;
        }
    };

    Ok(Value::record(out, span))
}

fn where_unmoved<'a>(
    record: &'a Record,
    column_idx: &'a [usize],
) -> impl Iterator<Item = (String, Value)> + use<'a> {
    record
        .iter()
        .enumerate()
        .filter(|(i, _)| column_idx.contains(i).not())
        .map(|(_, (c, v))| (c.clone(), v.clone()))
}

fn insert_moved(
    record: &Record,
    span: Span,
    column_idx: &[usize],
    out: &mut Record,
) -> Result<(), ShellError> {
    for idx in column_idx.iter() {
        if let Some((col, val)) = record.get_index(*idx) {
            out.push(col.clone(), val.clone());
        } else {
            return Err(ShellError::NushellFailedSpanned {
                msg: "Error indexing input columns".to_string(),
                label: "originates from here".to_string(),
                span,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    // helper
    fn get_test_record(columns: Vec<&str>, values: Vec<i64>) -> Record {
        let test_span = Span::test_data();
        Record::from_raw_cols_vals(
            columns.iter().map(|col| col.to_string()).collect(),
            values.iter().map(|val| Value::test_int(*val)).collect(),
            test_span,
            test_span,
        )
        .unwrap()
    }

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Move {})
    }

    #[test]
    fn move_after_with_single_column() {
        let test_span = Span::test_data();
        let test_record = get_test_record(vec!["a", "b", "c", "d"], vec![1, 2, 3, 4]);
        let after: Location = Location::After(Spanned {
            item: "c".to_string(),
            span: test_span,
        });
        let columns = ["a"].map(Value::test_string);

        // corresponds to: {a: 1, b: 2, c: 3, d: 4} | move a --after c
        let result = move_record_columns(&test_record, &columns, &after, test_span);

        assert!(result.is_ok());

        let result_record = result.unwrap().into_record().unwrap();
        let result_columns = result_record.into_columns().collect::<Vec<String>>();

        assert_eq!(result_columns, ["b", "c", "a", "d"]);
    }

    #[test]
    fn move_after_with_multiple_columns() {
        let test_span = Span::test_data();
        let test_record = get_test_record(vec!["a", "b", "c", "d", "e"], vec![1, 2, 3, 4, 5]);
        let after: Location = Location::After(Spanned {
            item: "c".to_string(),
            span: test_span,
        });
        let columns = ["b", "e"].map(Value::test_string);

        // corresponds to: {a: 1, b: 2, c: 3, d: 4, e: 5} | move b e --after c
        let result = move_record_columns(&test_record, &columns, &after, test_span);

        assert!(result.is_ok());

        let result_record = result.unwrap().into_record().unwrap();
        let result_columns = result_record.into_columns().collect::<Vec<String>>();

        assert_eq!(result_columns, ["a", "c", "b", "e", "d"]);
    }

    #[test]
    fn move_before_with_single_column() {
        let test_span = Span::test_data();
        let test_record = get_test_record(vec!["a", "b", "c", "d"], vec![1, 2, 3, 4]);
        let before: Location = Location::Before(Spanned {
            item: "b".to_string(),
            span: test_span,
        });
        let columns = ["d"].map(Value::test_string);

        // corresponds to: {a: 1, b: 2, c: 3, d: 4} | move d --before b
        let result = move_record_columns(&test_record, &columns, &before, test_span);

        assert!(result.is_ok());

        let result_record = result.unwrap().into_record().unwrap();
        let result_columns = result_record.into_columns().collect::<Vec<String>>();

        assert_eq!(result_columns, ["a", "d", "b", "c"]);
    }

    #[test]
    fn move_before_with_multiple_columns() {
        let test_span = Span::test_data();
        let test_record = get_test_record(vec!["a", "b", "c", "d", "e"], vec![1, 2, 3, 4, 5]);
        let before: Location = Location::Before(Spanned {
            item: "b".to_string(),
            span: test_span,
        });
        let columns = ["c", "e"].map(Value::test_string);

        // corresponds to: {a: 1, b: 2, c: 3, d: 4, e: 5} | move c e --before b
        let result = move_record_columns(&test_record, &columns, &before, test_span);

        assert!(result.is_ok());

        let result_record = result.unwrap().into_record().unwrap();
        let result_columns = result_record.into_columns().collect::<Vec<String>>();

        assert_eq!(result_columns, ["a", "c", "e", "b", "d"]);
    }

    #[test]
    fn move_first_with_single_column() {
        let test_span = Span::test_data();
        let test_record = get_test_record(vec!["a", "b", "c", "d"], vec![1, 2, 3, 4]);
        let columns = ["c"].map(Value::test_string);

        // corresponds to: {a: 1, b: 2, c: 3, d: 4} | move c --first
        let result = move_record_columns(&test_record, &columns, &Location::First, test_span);

        assert!(result.is_ok());

        let result_record = result.unwrap().into_record().unwrap();
        let result_columns = result_record.into_columns().collect::<Vec<String>>();

        assert_eq!(result_columns, ["c", "a", "b", "d"]);
    }

    #[test]
    fn move_first_with_multiple_columns() {
        let test_span = Span::test_data();
        let test_record = get_test_record(vec!["a", "b", "c", "d", "e"], vec![1, 2, 3, 4, 5]);
        let columns = ["c", "e"].map(Value::test_string);

        // corresponds to: {a: 1, b: 2, c: 3, d: 4, e: 5} | move c e --first
        let result = move_record_columns(&test_record, &columns, &Location::First, test_span);

        assert!(result.is_ok());

        let result_record = result.unwrap().into_record().unwrap();
        let result_columns = result_record.into_columns().collect::<Vec<String>>();

        assert_eq!(result_columns, ["c", "e", "a", "b", "d"]);
    }

    #[test]
    fn move_last_with_single_column() {
        let test_span = Span::test_data();
        let test_record = get_test_record(vec!["a", "b", "c", "d"], vec![1, 2, 3, 4]);
        let columns = ["b"].map(Value::test_string);

        // corresponds to: {a: 1, b: 2, c: 3, d: 4} | move b --last
        let result = move_record_columns(&test_record, &columns, &Location::Last, test_span);

        assert!(result.is_ok());

        let result_record = result.unwrap().into_record().unwrap();
        let result_columns = result_record.into_columns().collect::<Vec<String>>();

        assert_eq!(result_columns, ["a", "c", "d", "b"]);
    }

    #[test]
    fn move_last_with_multiple_columns() {
        let test_span = Span::test_data();
        let test_record = get_test_record(vec!["a", "b", "c", "d", "e"], vec![1, 2, 3, 4, 5]);
        let columns = ["c", "d"].map(Value::test_string);

        // corresponds to: {a: 1, b: 2, c: 3, d: 4, e: 5} | move c d --last
        let result = move_record_columns(&test_record, &columns, &Location::Last, test_span);

        assert!(result.is_ok());

        let result_record = result.unwrap().into_record().unwrap();
        let result_columns = result_record.into_columns().collect::<Vec<String>>();

        assert_eq!(result_columns, ["a", "b", "e", "c", "d"]);
    }

    #[test]
    fn move_with_multiple_columns_reorders_columns() {
        let test_span = Span::test_data();
        let test_record = get_test_record(vec!["a", "b", "c", "d", "e"], vec![1, 2, 3, 4, 5]);
        let after: Location = Location::After(Spanned {
            item: "e".to_string(),
            span: test_span,
        });
        let columns = ["d", "c", "a"].map(Value::test_string);

        // corresponds to: {a: 1, b: 2, c: 3, d: 4, e: 5} | move d c a --after e
        let result = move_record_columns(&test_record, &columns, &after, test_span);

        assert!(result.is_ok());

        let result_record = result.unwrap().into_record().unwrap();
        let result_columns = result_record.into_columns().collect::<Vec<String>>();

        assert_eq!(result_columns, ["b", "e", "d", "c", "a"]);
    }

    #[test]
    fn move_fails_when_pivot_not_present() {
        let test_span = Span::test_data();
        let test_record = get_test_record(vec!["a", "b"], vec![1, 2]);
        let before: Location = Location::Before(Spanned {
            item: "non-existent".to_string(),
            span: test_span,
        });
        let columns = ["a"].map(Value::test_string);

        let result = move_record_columns(&test_record, &columns, &before, test_span);

        assert!(result.is_err());
    }

    #[test]
    fn move_fails_when_column_not_present() {
        let test_span = Span::test_data();
        let test_record = get_test_record(vec!["a", "b"], vec![1, 2]);
        let before: Location = Location::Before(Spanned {
            item: "b".to_string(),
            span: test_span,
        });
        let columns = ["a", "non-existent"].map(Value::test_string);

        let result = move_record_columns(&test_record, &columns, &before, test_span);

        assert!(result.is_err());
    }

    #[test]
    fn move_fails_when_column_is_also_pivot() {
        let test_span = Span::test_data();
        let test_record = get_test_record(vec!["a", "b", "c", "d"], vec![1, 2, 3, 4]);
        let after: Location = Location::After(Spanned {
            item: "b".to_string(),
            span: test_span,
        });
        let columns = ["b", "d"].map(Value::test_string);

        let result = move_record_columns(&test_record, &columns, &after, test_span);

        assert!(result.is_err());
    }
}
