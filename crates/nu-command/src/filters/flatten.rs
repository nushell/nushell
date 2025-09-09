use indexmap::IndexMap;
use nu_engine::command_prelude::*;
use nu_protocol::ast::PathMember;

#[derive(Clone)]
pub struct Flatten;

impl Command for Flatten {
    fn name(&self) -> &str {
        "flatten"
    }

    fn signature(&self) -> Signature {
        Signature::build("flatten")
            .input_output_types(vec![
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Any)),
                ),
                (Type::record(), Type::table()),
            ])
            .rest(
                "rest",
                SyntaxShape::String,
                "Optionally flatten data by column.",
            )
            .switch("all", "flatten inner table one level out", Some('a'))
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Flatten the table."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        flatten(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "flatten a table",
                example: "[[N, u, s, h, e, l, l]] | flatten ",
                result: Some(Value::test_list(vec![
                    Value::test_string("N"),
                    Value::test_string("u"),
                    Value::test_string("s"),
                    Value::test_string("h"),
                    Value::test_string("e"),
                    Value::test_string("l"),
                    Value::test_string("l"),
                ])),
            },
            Example {
                description: "flatten a table, get the first item",
                example: "[[N, u, s, h, e, l, l]] | flatten | first",
                result: None, //Some(Value::test_string("N")),
            },
            Example {
                description: "flatten a column having a nested table",
                example: "[[origin, people]; [Ecuador, ([[name, meal]; ['Andres', 'arepa']])]] | flatten --all | get meal",
                result: None, //Some(Value::test_string("arepa")),
            },
            Example {
                description: "restrict the flattening by passing column names",
                example: "[[origin, crate, versions]; [World, ([[name]; ['nu-cli']]), ['0.21', '0.22']]] | flatten versions --all | last | get versions",
                result: None, //Some(Value::test_string("0.22")),
            },
            Example {
                description: "Flatten inner table",
                example: "{ a: b, d: [ 1 2 3 4 ], e: [ 4 3 ] } | flatten d --all",
                result: Some(Value::list(
                    vec![
                        Value::test_record(record! {
                                "a" => Value::test_string("b"),
                                "d" => Value::test_int(1),
                                "e" => Value::test_list(
                                    vec![Value::test_int(4), Value::test_int(3)],
                                ),
                        }),
                        Value::test_record(record! {
                                "a" => Value::test_string("b"),
                                "d" => Value::test_int(2),
                                "e" => Value::test_list(
                                    vec![Value::test_int(4), Value::test_int(3)],
                                ),
                        }),
                        Value::test_record(record! {
                                "a" => Value::test_string("b"),
                                "d" => Value::test_int(3),
                                "e" => Value::test_list(
                                    vec![Value::test_int(4), Value::test_int(3)],
                                ),
                        }),
                        Value::test_record(record! {
                                "a" => Value::test_string("b"),
                                "d" => Value::test_int(4),
                                "e" => Value::test_list(
                                    vec![Value::test_int(4), Value::test_int(3)],
                                )
                        }),
                    ],
                    Span::test_data(),
                )),
            },
        ]
    }
}

fn flatten(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let columns: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
    let metadata = input.metadata();
    let flatten_all = call.has_flag(engine_state, stack, "all")?;

    input
        .flat_map(
            move |item| flat_value(&columns, item, flatten_all),
            engine_state.signals(),
        )
        .map(|x| x.set_metadata(metadata))
}

enum TableInside {
    // handle for a column which contains a single list(but not list of records)
    // it contains (column, span, values in the column, column index).
    Entries(String, Vec<Value>, usize),
    // handle for a column which contains a table, we can flatten the inner column to outer level
    // `records` is the nested/inner table to flatten to the outer level
    // `parent_column_name` is handled for conflicting column name, the nested table may contains columns which has the same name
    // to outer level, for that case, the output column name should be f"{parent_column_name}_{inner_column_name}".
    // `parent_column_index` is the column index in original table.
    FlattenedRows {
        records: Vec<Record>,
        parent_column_name: String,
        parent_column_index: usize,
    },
}

fn flat_value(columns: &[CellPath], item: Value, all: bool) -> Vec<Value> {
    let tag = item.span();

    match item {
        Value::Record { val, .. } => {
            let mut out = IndexMap::<String, Value>::new();
            let mut inner_table = None;

            for (column_index, (column, value)) in val.into_owned().into_iter().enumerate() {
                let column_requested = columns.iter().find(|c| c.to_column_name() == column);
                let need_flatten = { columns.is_empty() || column_requested.is_some() };
                let span = value.span();

                match value {
                    Value::Record { ref val, .. } => {
                        if need_flatten {
                            for (col, val) in val.clone().into_owned() {
                                if out.contains_key(&col) {
                                    out.insert(format!("{column}_{col}"), val);
                                } else {
                                    out.insert(col, val);
                                }
                            }
                        } else if out.contains_key(&column) {
                            out.insert(format!("{column}_{column}"), value);
                        } else {
                            out.insert(column, value);
                        }
                    }
                    Value::List { vals, .. } => {
                        if need_flatten && inner_table.is_some() {
                            return vec![Value::error(
                                ShellError::UnsupportedInput {
                                    msg: "can only flatten one inner list at a time. tried flattening more than one column with inner lists... but is flattened already".into(),
                                    input: "value originates from here".into(),
                                    msg_span: tag,
                                    input_span: span
                                },
                                span,
                            )];
                        }

                        if all && vals.iter().all(|f| f.as_record().is_ok()) {
                            // it's a table (a list of record, we can flatten inner record)
                            if need_flatten {
                                let records = vals
                                    .into_iter()
                                    .filter_map(|v| v.into_record().ok())
                                    .collect();

                                inner_table = Some(TableInside::FlattenedRows {
                                    records,
                                    parent_column_name: column,
                                    parent_column_index: column_index,
                                });
                            } else if out.contains_key(&column) {
                                out.insert(format!("{column}_{column}"), Value::list(vals, span));
                            } else {
                                out.insert(column, Value::list(vals, span));
                            }
                        } else if !columns.is_empty() {
                            let cell_path =
                                column_requested.and_then(|x| match x.members.first() {
                                    Some(PathMember::String { val, .. }) => Some(val),
                                    _ => None,
                                });

                            if let Some(r) = cell_path {
                                inner_table =
                                    Some(TableInside::Entries(r.clone(), vals, column_index));
                            } else {
                                out.insert(column, Value::list(vals, span));
                            }
                        } else {
                            inner_table = Some(TableInside::Entries(column, vals, column_index));
                        }
                    }
                    _ => {
                        out.insert(column, value);
                    }
                }
            }

            let mut expanded = vec![];
            match inner_table {
                Some(TableInside::Entries(column, entries, parent_column_index)) => {
                    for entry in entries {
                        let base = out.clone();
                        let mut record = Record::new();
                        let mut index = 0;
                        for (col, val) in base.into_iter() {
                            // meet the flattened column, push them to result record first
                            // this can avoid output column order changed.
                            if index == parent_column_index {
                                record.push(column.clone(), entry.clone());
                            }
                            record.push(col, val);
                            index += 1;
                        }
                        // the flattened column may be the last column in the original table.
                        if index == parent_column_index {
                            record.push(column.clone(), entry);
                        }
                        expanded.push(Value::record(record, tag));
                    }
                }
                Some(TableInside::FlattenedRows {
                    records,
                    parent_column_name,
                    parent_column_index,
                }) => {
                    for inner_record in records {
                        let base = out.clone();
                        let mut record = Record::new();
                        let mut index = 0;

                        for (base_col, base_val) in base {
                            // meet the flattened column, push them to result record first
                            // this can avoid output column order changed.
                            if index == parent_column_index {
                                for (col, val) in &inner_record {
                                    if record.contains(col) {
                                        record.push(
                                            format!("{parent_column_name}_{col}"),
                                            val.clone(),
                                        );
                                    } else {
                                        record.push(col, val.clone());
                                    };
                                }
                            }

                            record.push(base_col, base_val);
                            index += 1;
                        }

                        // the flattened column may be the last column in the original table.
                        if index == parent_column_index {
                            for (col, val) in inner_record {
                                if record.contains(&col) {
                                    record.push(format!("{parent_column_name}_{col}"), val);
                                } else {
                                    record.push(col, val);
                                }
                            }
                        }
                        expanded.push(Value::record(record, tag));
                    }
                }
                None => {
                    expanded.push(Value::record(out.into_iter().collect(), tag));
                }
            }
            expanded
        }
        Value::List { vals, .. } => vals,
        item => vec![item],
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Flatten {})
    }
}
