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
                result: None
            },
            Example {
                description: "flatten a table, get the first item",
                example: "[[N, u, s, h, e, l, l]] | flatten | first",
                result: None,
            },
            Example {
                description: "flatten a column having a nested table",
                example: "[[origin, people]; [Ecuador, ([[name, meal]; ['Andres', 'arepa']])]] | flatten | get meal",
                result: None,
            },
            Example {
                description: "restrict the flattening by passing column names",
                example: "[[origin, crate, versions]; [World, ([[name]; ['nu-cli']]), ['0.21', '0.22']]] | flatten versions | last | get versions",
                result: None,
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

    input.flat_map(
        move |item| flat_value(&columns, &item, tag),
        engine_state.ctrlc.clone(),
    )
}

enum TableInside<'a> {
    Entries(&'a str, &'a Span, Vec<&'a Value>),
}

fn is_table(value: &Value) -> bool {
    match value {
        Value::List { vals, span: _ } => vals.iter().all(|f| f.as_record().is_ok()),
        _ => false,
    }
}

fn flat_value(columns: &[CellPath], item: &Value, _name_tag: Span) -> Vec<Value> {
    let tag = match item.span() {
        Ok(x) => x,
        Err(e) => return vec![Value::Error { error: e }],
    };

    let res = {
        if item.as_record().is_ok() {
            let mut out = IndexMap::<String, Value>::new();
            let mut a_table = None;
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

            for (column, value) in records.0.iter().zip(records.1.iter()) {
                let column_requested = columns.iter().find(|c| c.into_string() == *column);

                match value {
                    Value::List { vals, span: _ } if vals.iter().all(|f| f.as_record().is_ok()) => {
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
                            continue;
                        }

                        let cols = cs.into_iter().flat_map(|f| f.to_vec());
                        let vals = vs.into_iter().flat_map(|f| f.to_vec());

                        for (k, v) in cols.into_iter().zip(vals.into_iter()) {
                            if out.contains_key(&k) {
                                out.insert(format!("{}_{}", column.to_string(), k), v.clone());
                            } else {
                                out.insert(k, v.clone());
                            }
                        }
                    }
                    Value::List { vals: _, span: _ } => {
                        let vals = if let Value::List { vals, span: _ } = value {
                            vals.iter().collect::<Vec<_>>()
                        } else {
                            vec![]
                        };

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
                                    a_table = Some(TableInside::Entries(
                                        r,
                                        &s,
                                        vals.into_iter().collect::<Vec<_>>(),
                                    ));

                                    tables_explicitly_flattened += 1;
                                }
                            } else {
                                out.insert(column.to_string(), value.clone());
                            }
                        } else if a_table.is_none() {
                            a_table = Some(TableInside::Entries(
                                column,
                                &s,
                                vals.into_iter().collect::<Vec<_>>(),
                            ))
                        }
                    }
                    _ => {
                        out.insert(column.to_string(), value.clone());
                    }
                }
            }

            let mut expanded = vec![];

            if let Some(TableInside::Entries(column, _, entries)) = a_table {
                for entry in entries {
                    let mut base = out.clone();
                    base.insert(column.to_string(), entry.clone());
                    let r = Value::Record {
                        cols: base.keys().map(|f| f.to_string()).collect::<Vec<_>>(),
                        vals: base.values().cloned().collect(),
                        span: tag,
                    };
                    expanded.push(r);
                }
            } else {
                let r = Value::Record {
                    cols: out.keys().map(|f| f.to_string()).collect::<Vec<_>>(),
                    vals: out.values().cloned().collect(),
                    span: tag,
                };
                expanded.push(r);
            }
            expanded
        } else if !is_table(item) {
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
