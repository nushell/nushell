use nu_engine::command_prelude::*;

#[derive(Clone, Debug)]
enum BeforeOrAfter {
    Before(String),
    After(String),
}

#[derive(Clone)]
pub struct Move;

impl Command for Move {
    fn name(&self) -> &str {
        "move"
    }

    fn usage(&self) -> &str {
        "Move columns before or after other columns."
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
            .category(Category::Filters)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "[[name value index]; [foo a 1] [bar b 2] [baz c 3]] | move index --before name",
                description: "Move a column before the first column",
                result:
                    Some(Value::test_list(
                        vec![
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
                        ],
                    ))
            },
            Example {
                example: "[[name value index]; [foo a 1] [bar b 2] [baz c 3]] | move value name --after index",
                description: "Move multiple columns after the last column and reorder them",
                result:
                    Some(Value::test_list(
                        vec![
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
                        ],
                    ))
            },
            Example {
                example: "{ name: foo, value: a, index: 1 } | move name --before index",
                description: "Move columns of a record",
                result: Some(Value::test_record(record! {
                    "value" => Value::test_string("a"),
                    "name" => Value::test_string("foo"),
                    "index" => Value::test_int(1),
                }))
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
        let columns: Vec<Value> = call.rest(engine_state, stack, 0)?;
        let after: Option<Value> = call.get_flag(engine_state, stack, "after")?;
        let before: Option<Value> = call.get_flag(engine_state, stack, "before")?;

        let before_or_after = match (after, before) {
            (Some(v), None) => Spanned {
                span: v.span(),
                item: BeforeOrAfter::After(v.coerce_into_string()?),
            },
            (None, Some(v)) => Spanned {
                span: v.span(),
                item: BeforeOrAfter::Before(v.coerce_into_string()?),
            },
            (Some(_), Some(_)) => {
                return Err(ShellError::GenericError {
                    error: "Cannot move columns".into(),
                    msg: "Use either --after, or --before, not both".into(),
                    span: Some(call.head),
                    help: None,
                    inner: vec![],
                })
            }
            (None, None) => {
                return Err(ShellError::GenericError {
                    error: "Cannot move columns".into(),
                    msg: "Missing --after or --before flag".into(),
                    span: Some(call.head),
                    help: None,
                    inner: vec![],
                })
            }
        };

        let metadata = input.metadata();
        let ctrlc = engine_state.ctrlc.clone();
        let call = call.clone();

        match input {
            PipelineData::Value(Value::List { .. }, ..) | PipelineData::ListStream { .. } => {
                let res = input.into_iter().map(move |x| match x.as_record() {
                    Ok(record) => {
                        match move_record_columns(record, &columns, &before_or_after, call.head) {
                            Ok(val) => val,
                            Err(error) => Value::error(error, call.head),
                        }
                    }
                    Err(error) => Value::error(error, call.head),
                });

                if let Some(md) = metadata {
                    Ok(res.into_pipeline_data_with_metadata(md, ctrlc))
                } else {
                    Ok(res.into_pipeline_data(ctrlc))
                }
            }
            PipelineData::Value(Value::Record { val, .. }, ..) => {
                Ok(
                    move_record_columns(&val, &columns, &before_or_after, call.head)?
                        .into_pipeline_data(),
                )
            }
            _ => Err(ShellError::PipelineMismatch {
                exp_input_type: "record or table".to_string(),
                dst_span: call.head,
                src_span: Span::new(call.head.start, call.head.start),
            }),
        }
    }
}

// Move columns within a record
fn move_record_columns(
    record: &Record,
    columns: &[Value],
    before_or_after: &Spanned<BeforeOrAfter>,
    span: Span,
) -> Result<Value, ShellError> {
    let mut column_idx: Vec<usize> = Vec::with_capacity(columns.len());

    let pivot = match &before_or_after.item {
        BeforeOrAfter::Before(before) => before,
        BeforeOrAfter::After(after) => after,
    };

    // check if pivot exists
    if !record.contains(pivot) {
        return Err(ShellError::GenericError {
            error: "Cannot move columns".into(),
            msg: "column does not exist".into(),
            span: Some(before_or_after.span),
            help: None,
            inner: vec![],
        });
    }

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

        let column_as_string = column.coerce_string()?;
        // check if column is also pivot
        if &column_as_string == pivot {
            return Err(ShellError::IncompatibleParameters {
                left_message: "Column cannot be moved".to_string(),
                left_span: column.span(),
                right_message: "relative to itself".to_string(),
                right_span: before_or_after.span,
            });
        }
    }

    let mut out = Record::with_capacity(record.len());

    for (i, (inp_col, inp_val)) in record.iter().enumerate() {
        if inp_col == pivot {
            if matches!(&before_or_after.item, BeforeOrAfter::After(..)) {
                out.push(inp_col.clone(), inp_val.clone());
            }

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

            if matches!(&before_or_after.item, BeforeOrAfter::Before(..)) {
                out.push(inp_col.clone(), inp_val.clone());
            }
        } else if !column_idx.contains(&i) {
            out.push(inp_col.clone(), inp_val.clone());
        }
    }

    Ok(Value::record(out, span))
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
        let after: Spanned<BeforeOrAfter> = Spanned {
            item: BeforeOrAfter::After("c".to_string()),
            span: test_span,
        };
        let columns = [Value::test_string("a")];

        // corresponds to: {a: 1, b: 2, c: 3, d: 4} | move a --after c
        let result = move_record_columns(&test_record, &columns, &after, test_span);

        assert!(result.is_ok());

        let result_rec_tmp = result.unwrap();
        let result_record = result_rec_tmp.as_record().unwrap();

        assert_eq!(*result_record.get_index(0).unwrap().0, "b".to_string());
        assert_eq!(*result_record.get_index(1).unwrap().0, "c".to_string());
        assert_eq!(*result_record.get_index(2).unwrap().0, "a".to_string());
        assert_eq!(*result_record.get_index(3).unwrap().0, "d".to_string());
    }

    #[test]
    fn move_after_with_multiple_columns() {
        let test_span = Span::test_data();
        let test_record = get_test_record(vec!["a", "b", "c", "d", "e"], vec![1, 2, 3, 4, 5]);
        let after: Spanned<BeforeOrAfter> = Spanned {
            item: BeforeOrAfter::After("c".to_string()),
            span: test_span,
        };
        let columns = [Value::test_string("b"), Value::test_string("e")];

        // corresponds to: {a: 1, b: 2, c: 3, d: 4, e: 5} | move b e --after c
        let result = move_record_columns(&test_record, &columns, &after, test_span);

        assert!(result.is_ok());

        let result_rec_tmp = result.unwrap();
        let result_record = result_rec_tmp.as_record().unwrap();

        assert_eq!(*result_record.get_index(0).unwrap().0, "a".to_string());
        assert_eq!(*result_record.get_index(1).unwrap().0, "c".to_string());
        assert_eq!(*result_record.get_index(2).unwrap().0, "b".to_string());
        assert_eq!(*result_record.get_index(3).unwrap().0, "e".to_string());
        assert_eq!(*result_record.get_index(4).unwrap().0, "d".to_string());
    }

    #[test]
    fn move_before_with_single_column() {
        let test_span = Span::test_data();
        let test_record = get_test_record(vec!["a", "b", "c", "d"], vec![1, 2, 3, 4]);
        let after: Spanned<BeforeOrAfter> = Spanned {
            item: BeforeOrAfter::Before("b".to_string()),
            span: test_span,
        };
        let columns = [Value::test_string("d")];

        // corresponds to: {a: 1, b: 2, c: 3, d: 4} | move d --before b
        let result = move_record_columns(&test_record, &columns, &after, test_span);

        assert!(result.is_ok());

        let result_rec_tmp = result.unwrap();
        let result_record = result_rec_tmp.as_record().unwrap();

        assert_eq!(*result_record.get_index(0).unwrap().0, "a".to_string());
        assert_eq!(*result_record.get_index(1).unwrap().0, "d".to_string());
        assert_eq!(*result_record.get_index(2).unwrap().0, "b".to_string());
        assert_eq!(*result_record.get_index(3).unwrap().0, "c".to_string());
    }

    #[test]
    fn move_before_with_multiple_columns() {
        let test_span = Span::test_data();
        let test_record = get_test_record(vec!["a", "b", "c", "d", "e"], vec![1, 2, 3, 4, 5]);
        let after: Spanned<BeforeOrAfter> = Spanned {
            item: BeforeOrAfter::Before("b".to_string()),
            span: test_span,
        };
        let columns = [Value::test_string("c"), Value::test_string("e")];

        // corresponds to: {a: 1, b: 2, c: 3, d: 4, e: 5} | move c e --before b
        let result = move_record_columns(&test_record, &columns, &after, test_span);

        assert!(result.is_ok());

        let result_rec_tmp = result.unwrap();
        let result_record = result_rec_tmp.as_record().unwrap();

        assert_eq!(*result_record.get_index(0).unwrap().0, "a".to_string());
        assert_eq!(*result_record.get_index(1).unwrap().0, "c".to_string());
        assert_eq!(*result_record.get_index(2).unwrap().0, "e".to_string());
        assert_eq!(*result_record.get_index(3).unwrap().0, "b".to_string());
        assert_eq!(*result_record.get_index(4).unwrap().0, "d".to_string());
    }

    #[test]
    fn move_with_multiple_columns_reorders_columns() {
        let test_span = Span::test_data();
        let test_record = get_test_record(vec!["a", "b", "c", "d", "e"], vec![1, 2, 3, 4, 5]);
        let after: Spanned<BeforeOrAfter> = Spanned {
            item: BeforeOrAfter::After("e".to_string()),
            span: test_span,
        };
        let columns = [
            Value::test_string("d"),
            Value::test_string("c"),
            Value::test_string("a"),
        ];

        // corresponds to: {a: 1, b: 2, c: 3, d: 4, e: 5} | move d c a --after e
        let result = move_record_columns(&test_record, &columns, &after, test_span);

        assert!(result.is_ok());

        let result_rec_tmp = result.unwrap();
        let result_record = result_rec_tmp.as_record().unwrap();

        assert_eq!(*result_record.get_index(0).unwrap().0, "b".to_string());
        assert_eq!(*result_record.get_index(1).unwrap().0, "e".to_string());
        assert_eq!(*result_record.get_index(2).unwrap().0, "d".to_string());
        assert_eq!(*result_record.get_index(3).unwrap().0, "c".to_string());
        assert_eq!(*result_record.get_index(4).unwrap().0, "a".to_string());
    }

    #[test]
    fn move_fails_when_pivot_not_present() {
        let test_span = Span::test_data();
        let test_record = get_test_record(vec!["a", "b"], vec![1, 2]);
        let after: Spanned<BeforeOrAfter> = Spanned {
            item: BeforeOrAfter::Before("non-existent".to_string()),
            span: test_span,
        };
        let columns = [Value::test_string("a")];

        let result = move_record_columns(&test_record, &columns, &after, test_span);

        assert!(result.is_err());
    }

    #[test]
    fn move_fails_when_column_not_present() {
        let test_span = Span::test_data();
        let test_record = get_test_record(vec!["a", "b"], vec![1, 2]);
        let after: Spanned<BeforeOrAfter> = Spanned {
            item: BeforeOrAfter::Before("b".to_string()),
            span: test_span,
        };
        let columns = [Value::test_string("a"), Value::test_string("non-existent")];

        let result = move_record_columns(&test_record, &columns, &after, test_span);

        assert!(result.is_err());
    }

    #[test]
    fn move_fails_when_column_is_also_pivot() {
        let test_span = Span::test_data();
        let test_record = get_test_record(vec!["a", "b", "c", "d"], vec![1, 2, 3, 4]);
        let after: Spanned<BeforeOrAfter> = Spanned {
            item: BeforeOrAfter::After("b".to_string()),
            span: test_span,
        };
        let columns = [Value::test_string("b"), Value::test_string("d")];

        let result = move_record_columns(&test_record, &columns, &after, test_span);

        assert!(result.is_err());
    }
}
