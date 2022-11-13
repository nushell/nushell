use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct SplitBy;

impl Command for SplitBy {
    fn name(&self) -> &str {
        "split-by"
    }

    fn signature(&self) -> Signature {
        Signature::build("split-by")
            .input_output_types(vec![(Type::Record(vec![]), Type::Record(vec![]))])
            .optional("splitter", SyntaxShape::Any, "the splitter value to use")
    }

    fn usage(&self) -> &str {
        "Create a new table splitted."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        split_by(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "split items by column named \"lang\"",
            example: r#"
                {
                    '2019': [
                      { name: 'andres', lang: 'rb', year: '2019' },
                      { name: 'jt', lang: 'rs', year: '2019' }
                    ],
                    '2021': [
                      { name: 'storm', lang: 'rs', 'year': '2021' }
                    ]
                } | split-by lang
                "#,
            result: Some(Value::Record {
                cols: vec!["rb".to_string(), "rs".to_string()],
                vals: vec![
                    Value::Record {
                        cols: vec!["2019".to_string()],
                        vals: vec![Value::List {
                            vals: vec![Value::Record {
                                cols: vec![
                                    "name".to_string(),
                                    "lang".to_string(),
                                    "year".to_string(),
                                ],
                                vals: vec![
                                    Value::test_string("andres"),
                                    Value::test_string("rb"),
                                    Value::test_string("2019"),
                                ],
                                span: Span::test_data(),
                            }],
                            span: Span::test_data(),
                        }],
                        span: Span::test_data(),
                    },
                    Value::Record {
                        cols: vec!["2019".to_string(), "2021".to_string()],
                        vals: vec![
                            Value::List {
                                vals: vec![Value::Record {
                                    cols: vec![
                                        "name".to_string(),
                                        "lang".to_string(),
                                        "year".to_string(),
                                    ],
                                    vals: vec![
                                        Value::test_string("jt"),
                                        Value::test_string("rs"),
                                        Value::test_string("2019"),
                                    ],
                                    span: Span::test_data(),
                                }],
                                span: Span::test_data(),
                            },
                            Value::List {
                                vals: vec![Value::Record {
                                    cols: vec![
                                        "name".to_string(),
                                        "lang".to_string(),
                                        "year".to_string(),
                                    ],
                                    vals: vec![
                                        Value::test_string("storm"),
                                        Value::test_string("rs"),
                                        Value::test_string("2021"),
                                    ],
                                    span: Span::test_data(),
                                }],
                                span: Span::test_data(),
                            },
                        ],
                        span: Span::test_data(),
                    },
                ],
                span: Span::test_data(),
            }),
        }]
    }
}

enum Grouper {
    ByColumn(Option<Spanned<String>>),
}

pub fn split_by(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let name = call.head;

    let splitter: Option<Value> = call.opt(engine_state, stack, 0)?;

    match splitter {
        Some(v) => {
            let splitter = Some(Spanned {
                item: v.as_string()?,
                span: name,
            });
            Ok(split(&splitter, input, name)?)
        }
        None => Err(ShellError::GenericError(
            "expected name".into(),
            "requires a column name for splitting".into(),
            Some(name),
            None,
            Vec::new(),
        )),
    }
}

pub fn split(
    column_name: &Option<Spanned<String>>,
    values: PipelineData,
    span: Span,
) -> Result<PipelineData, ShellError> {
    let grouper = if let Some(column_name) = column_name {
        Grouper::ByColumn(Some(column_name.clone()))
    } else {
        Grouper::ByColumn(None)
    };

    match grouper {
        Grouper::ByColumn(Some(column_name)) => {
            let block =
                Box::new(
                    move |_, row: &Value| match row.get_data_by_key(&column_name.item) {
                        Some(group_key) => Ok(group_key.as_string()?),
                        None => Err(ShellError::CantFindColumn(
                            column_name.span,
                            row.span().unwrap_or(column_name.span),
                        )),
                    },
                );

            data_split(values, &Some(block), span)
        }
        Grouper::ByColumn(None) => {
            let block = Box::new(move |_, row: &Value| row.as_string());

            data_split(values, &Some(block), span)
        }
    }
}

#[allow(clippy::type_complexity)]
pub fn data_split(
    value: PipelineData,
    splitter: &Option<Box<dyn Fn(usize, &Value) -> Result<String, ShellError> + Send>>,
    span: Span,
) -> Result<PipelineData, ShellError> {
    let mut splits = indexmap::IndexMap::new();

    let mut cols = vec![];
    let mut vals = vec![];

    match value {
        PipelineData::Value(
            Value::Record {
                cols,
                vals: grouped_rows,
                span,
            },
            _,
        ) => {
            for (idx, list) in grouped_rows.iter().enumerate() {
                match super::group_by::data_group(list, splitter, span) {
                    Ok(grouped) => {
                        if let Value::Record {
                            vals: li,
                            cols: sub_cols,
                            ..
                        } = grouped
                        {
                            for (inner_idx, subset) in li.iter().enumerate() {
                                let s = splits
                                    .entry(sub_cols[inner_idx].clone())
                                    .or_insert(indexmap::IndexMap::new());

                                s.insert(cols[idx].clone(), subset.clone());
                            }
                        }
                    }
                    Err(reason) => return Err(reason),
                }
            }
        }
        _ => {
            return Err(ShellError::GenericError(
                "unsupported input".into(),
                "requires a table with one row for splitting".into(),
                Some(span),
                None,
                Vec::new(),
            ))
        }
    }

    for (k, rows) in splits {
        cols.push(k.to_string());

        let mut sub_cols = vec![];
        let mut sub_vals = vec![];

        for (k, v) in rows {
            sub_cols.push(k);
            sub_vals.push(v);
        }

        vals.push(Value::Record {
            cols: sub_cols,
            vals: sub_vals,
            span,
        });
    }

    Ok(PipelineData::Value(
        Value::Record { cols, vals, span },
        None,
    ))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SplitBy {})
    }
}
