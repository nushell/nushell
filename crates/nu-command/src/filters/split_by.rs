use indexmap::IndexMap;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, Record, ShellError, Signature, Span,
    Spanned, SyntaxShape, Type, Value,
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
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Create a new table split."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        split_by(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "split items by column named \"lang\"",
            example: r#"{
        '2019': [
          { name: 'andres', lang: 'rb', year: '2019' },
          { name: 'jt', lang: 'rs', year: '2019' }
        ],
        '2021': [
          { name: 'storm', lang: 'rs', 'year': '2021' }
        ]
    } | split-by lang"#,
            result: Some(Value::test_record(Record {
                cols: vec!["rb".to_string(), "rs".to_string()],
                vals: vec![
                    Value::test_record(Record {
                        cols: vec!["2019".to_string()],
                        vals: vec![Value::list(
                            vec![Value::test_record(Record {
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
                            })],
                            Span::test_data(),
                        )],
                    }),
                    Value::test_record(Record {
                        cols: vec!["2019".to_string(), "2021".to_string()],
                        vals: vec![
                            Value::list(
                                vec![Value::test_record(Record {
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
                                })],
                                Span::test_data(),
                            ),
                            Value::list(
                                vec![Value::test_record(Record {
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
                                })],
                                Span::test_data(),
                            ),
                        ],
                    }),
                ],
            })),
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
            Ok(split(splitter.as_ref(), input, name)?)
        }
        // This uses the same format as the 'requires a column name' error in sort_utils.rs
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
    column_name: Option<&Spanned<String>>,
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
            let block = move |_, row: &Value| match row.get_data_by_key(&column_name.item) {
                Some(group_key) => Ok(group_key.as_string()?),
                None => Err(ShellError::CantFindColumn {
                    col_name: column_name.item.to_string(),
                    span: column_name.span,
                    src_span: row.span(),
                }),
            };

            data_split(values, Some(&block), span)
        }
        Grouper::ByColumn(None) => {
            let block = move |_, row: &Value| row.as_string();

            data_split(values, Some(&block), span)
        }
    }
}

#[allow(clippy::type_complexity)]
fn data_group(
    values: &Value,
    grouper: Option<&dyn Fn(usize, &Value) -> Result<String, ShellError>>,
    span: Span,
) -> Result<Value, ShellError> {
    let mut groups: IndexMap<String, Vec<Value>> = IndexMap::new();

    for (idx, value) in values.clone().into_pipeline_data().into_iter().enumerate() {
        let group_key = if let Some(ref grouper) = grouper {
            grouper(idx, &value)
        } else {
            value.as_string()
        };

        let group = groups.entry(group_key?).or_default();
        group.push(value);
    }

    Ok(Value::record(
        groups
            .into_iter()
            .map(|(k, v)| (k, Value::list(v, span)))
            .collect(),
        span,
    ))
}

#[allow(clippy::type_complexity)]
pub fn data_split(
    value: PipelineData,
    splitter: Option<&dyn Fn(usize, &Value) -> Result<String, ShellError>>,
    span: Span,
) -> Result<PipelineData, ShellError> {
    let mut splits = indexmap::IndexMap::new();

    match value {
        PipelineData::Value(v, _) => {
            let span = v.span();
            match v {
                Value::Record { val: grouped, .. } => {
                    for (idx, list) in grouped.vals.iter().enumerate() {
                        match data_group(list, splitter, span) {
                            Ok(grouped_vals) => {
                                if let Value::Record { val: sub, .. } = grouped_vals {
                                    for (inner_idx, subset) in sub.vals.iter().enumerate() {
                                        let s: &mut IndexMap<String, Value> =
                                            splits.entry(sub.cols[inner_idx].clone()).or_default();

                                        s.insert(grouped.cols[idx].clone(), subset.clone());
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

    let record = splits
        .into_iter()
        .map(|(k, rows)| (k, Value::record(rows.into_iter().collect(), span)))
        .collect();

    Ok(PipelineData::Value(Value::record(record, span), None))
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
