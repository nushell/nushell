use indexmap::IndexMap;
use nu_engine::CallExt;
use nu_protocol::ast::{Call, CellPath, PathMember};

use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct Flatten;

impl Command for Flatten {
    fn name(&self) -> &str {
        "flatten"
    }

    fn signature(&self) -> Signature {
        Signature::build("flatten")
            .rest(
                "rest",
                SyntaxShape::String,
                "optionally flatten data by column",
            )
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Flatten the table."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        flatten(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "flatten a table",
                example: "[[N, u, s, h, e, l, l]] | flatten ",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_string("N"),
                        Value::test_string("u"),
                        Value::test_string("s"),
                        Value::test_string("h"),
                        Value::test_string("e"),
                        Value::test_string("l"),
                        Value::test_string("l")],
                    span: Span::test_data()
                })
            },
            Example {
                description: "flatten a table, get the first item",
                example: "[[N, u, s, h, e, l, l]] | flatten | first",
                result: None,//Some(Value::test_string("N")),
            },
            Example {
                description: "flatten a column having a nested table",
                example: "[[origin, people]; [Ecuador, ([[name, meal]; ['Andres', 'arepa']])]] | flatten | get meal",
                result: None,//Some(Value::test_string("arepa")),
            },
            Example {
                description: "restrict the flattening by passing column names",
                example: "[[origin, crate, versions]; [World, ([[name]; ['nu-cli']]), ['0.21', '0.22']]] | flatten versions | last | get versions",
                result: None, //Some(Value::test_string("0.22")),
            },
            Example {
                description: "Flatten inner table",
                example: "{ a: b, d: [ 1 2 3 4 ],  e: [ 4 3  ] } | flatten",
                result: Some(Value::List{
                    vals: vec![
                        Value::Record{
                            cols: vec!["a".to_string(), "d".to_string(), "e".to_string()],
                            vals: vec![Value::test_string("b"), Value::test_int(1), Value::List{vals: vec![Value::test_int(4), Value::test_int(3)], span: Span::test_data()}                            ],
                            span: Span::test_data()
                        },
                        Value::Record{
                            cols: vec!["a".to_string(), "d".to_string(), "e".to_string()],
                            vals: vec![Value::test_string("b"), Value::test_int(2), Value::List{vals: vec![Value::test_int(4), Value::test_int(3)], span: Span::test_data()}                            ],
                            span: Span::test_data()
                        },
                        Value::Record{
                            cols: vec!["a".to_string(), "d".to_string(), "e".to_string()],
                            vals: vec![Value::test_string("b"), Value::test_int(3), Value::List{vals: vec![Value::test_int(4), Value::test_int(3)], span: Span::test_data()}                            ],
                            span: Span::test_data()
                        },
                        Value::Record{
                            cols: vec!["a".to_string(), "d".to_string(), "e".to_string()],
                            vals: vec![Value::test_string("b"), Value::test_int(4), Value::List{vals: vec![Value::test_int(4), Value::test_int(3)], span: Span::test_data()}                            ],
                            span: Span::test_data()
                        }
                    ],
                    span: Span::test_data(),
                }),
            }
        ]
    }
}

fn flatten(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
    let tag = call.head;
    let columns: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
    let metadata = input.metadata();

    input
        .flat_map(
            move |item| flat_value(&columns, &item, tag),
            engine_state.ctrlc.clone(),
        )
        .map(|x| x.set_metadata(metadata))
}

enum TableInside<'a> {
    // handle for a column which contains a single list(but not list of records).
    Entries(&'a str, &'a Span, Vec<&'a Value>),
    // handle for a column which contains a table, we can flatten the inner column to outer level
    // `columns` means that for the given row, it contains `len(columns)` nested rows, and each nested row contains a list of column name.
    // Likely, `values` means that for the given row, it contains `len(values)` nested rows, and each nested row contains a list of values.
    //
    // `parent_column_name` is handled for conflicting column name, the nested table may contains columns which has the same name
    // to outer level, for that case, the output column name should be f"{parent_column_name}_{inner_column_name}".
    FlatternedRows {
        columns: Vec<Vec<String>>,
        _span: &'a Span,
        values: Vec<Vec<Value>>,
        parent_column_name: &'a str,
    },
}

fn flat_value(columns: &[CellPath], item: &Value, _name_tag: Span) -> Vec<Value> {
    let tag = match item.span() {
        Ok(x) => x,
        Err(e) => return vec![Value::Error { error: e }],
    };

    let res = {
        if item.as_record().is_ok() {
            let mut out = IndexMap::<String, Value>::new();
            let mut inner_table = None;
            let mut tables_explicitly_flattened = 0;

            let records = match item {
                Value::Record {
                    cols,
                    vals,
                    span: _,
                } => (cols, vals),
                x => {
                    return vec![Value::Error {
                        error: ShellError::UnsupportedInput(
                            format!("This should be a record, but instead got {}", x.get_type()),
                            tag,
                        ),
                    }]
                }
            };

            let s = match item.span() {
                Ok(x) => x,
                Err(e) => return vec![Value::Error { error: e }],
            };

            let records_iterator = {
                let cols = records.0;
                let vals = records.1;

                let mut pairs = vec![];
                for i in 0..cols.len() {
                    pairs.push((cols[i].as_str(), &vals[i]));
                }
                pairs
            };

            for (column, value) in records_iterator {
                let column_requested = columns.iter().find(|c| c.into_string() == *column);

                match value {
                    Value::Record {
                        cols,
                        vals,
                        span: _,
                    } => {
                        if column_requested.is_none() && !columns.is_empty() {
                            if out.contains_key(column) {
                                out.insert(format!("{}_{}", column, column), value.clone());
                            } else {
                                out.insert(column.to_string(), value.clone());
                            }
                        } else {
                            cols.iter().enumerate().for_each(|(idx, inner_record_col)| {
                                if out.contains_key(inner_record_col) {
                                    out.insert(
                                        format!("{}_{}", column, inner_record_col),
                                        vals[idx].clone(),
                                    );
                                } else {
                                    out.insert(inner_record_col.to_string(), vals[idx].clone());
                                }
                            })
                        }
                    }
                    Value::List { vals, span: _ } if vals.iter().all(|f| f.as_record().is_ok()) => {
                        // it's a table (a list of record, we can flatten inner record)
                        let mut cs = vec![];
                        let mut vs = vec![];

                        for v in vals {
                            if let Ok(r) = v.as_record() {
                                cs.push(r.0);
                                vs.push(r.1)
                            }
                        }

                        if column_requested.is_none() && !columns.is_empty() {
                            if out.contains_key(column) {
                                out.insert(format!("{}_{}", column, column), value.clone());
                            } else {
                                out.insert(column.to_string(), value.clone());
                            }
                        } else {
                            let cols = cs.into_iter().map(|f| f.to_vec());
                            let vals = vs.into_iter().map(|f| f.to_vec());

                            inner_table = Some(TableInside::FlatternedRows {
                                columns: cols.collect(),
                                _span: &s,
                                values: vals.collect(),
                                parent_column_name: column,
                            });
                        }
                    }
                    Value::List {
                        vals: values,
                        span: _,
                    } => {
                        if tables_explicitly_flattened >= 1 && column_requested.is_some() {
                            return vec![Value::Error{ error: ShellError::UnsupportedInput(
                                    "can only flatten one inner table at the same time. tried flattening more than one column with inner tables... but is flattened already".to_string(),
                                    s
                                )}
                            ];
                        }

                        if !columns.is_empty() {
                            let cell_path = match column_requested {
                                Some(x) => match x.members.first() {
                                    Some(PathMember::String { val, span: _ }) => Some(val),
                                    Some(PathMember::Int { val: _, span: _ }) => None,
                                    None => None,
                                },
                                None => None,
                            };

                            if let Some(r) = cell_path {
                                if !columns.is_empty() {
                                    inner_table = Some(TableInside::Entries(
                                        r,
                                        &s,
                                        values.iter().collect::<Vec<_>>(),
                                    ));

                                    tables_explicitly_flattened += 1;
                                }
                            } else {
                                out.insert(column.to_string(), value.clone());
                            }
                        } else if inner_table.is_none() {
                            inner_table = Some(TableInside::Entries(
                                column,
                                &s,
                                values.iter().collect::<Vec<_>>(),
                            ));
                            out.insert(column.to_string(), value.clone());
                        } else {
                            out.insert(column.to_string(), value.clone());
                        }
                    }
                    _ => {
                        out.insert(column.to_string(), value.clone());
                    }
                }
            }

            let mut expanded = vec![];
            match inner_table {
                Some(TableInside::Entries(column, _, entries)) => {
                    for entry in entries {
                        let mut base = out.clone();

                        base.insert(column.to_string(), entry.clone());
                        let record = Value::Record {
                            cols: base.keys().map(|f| f.to_string()).collect::<Vec<_>>(),
                            vals: base.values().cloned().collect(),
                            span: tag,
                        };
                        expanded.push(record);
                    }
                }
                Some(TableInside::FlatternedRows {
                    columns,
                    _span,
                    values,
                    parent_column_name,
                }) => {
                    for (inner_cols, inner_vals) in columns.into_iter().zip(values) {
                        let mut base = out.clone();
                        for (col, val) in inner_cols.into_iter().zip(inner_vals) {
                            if base.contains_key(&col) {
                                base.insert(format!("{}_{}", parent_column_name, col), val);
                            } else {
                                base.insert(col, val);
                            }
                        }
                        let record = Value::Record {
                            cols: base.keys().map(|f| f.to_string()).collect::<Vec<_>>(),
                            vals: base.values().cloned().collect(),
                            span: tag,
                        };
                        expanded.push(record);
                    }
                }
                None => {
                    let record = Value::Record {
                        cols: out.keys().map(|f| f.to_string()).collect::<Vec<_>>(),
                        vals: out.values().cloned().collect(),
                        span: tag,
                    };
                    expanded.push(record);
                }
            }
            expanded
        } else if item.as_list().is_ok() {
            if let Value::List { vals, span: _ } = item {
                vals.to_vec()
            } else {
                vec![]
            }
        } else {
            vec![item.clone()]
        }
    };
    res
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
