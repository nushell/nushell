use indexmap::IndexMap;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct SplitBy;

impl Command for SplitBy {
    fn name(&self) -> &str {
        "split-by"
    }

    fn signature(&self) -> Signature {
        Signature::build("split-by")
            .input_output_types(vec![(Type::record(), Type::record())])
            .optional("splitter", SyntaxShape::Any, "The splitter value to use.")
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Split a record into groups."
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
            result: Some(Value::test_record(record! {
                    "rb" => Value::test_record(record! {
                        "2019" => Value::test_list(
                            vec![Value::test_record(record! {
                                    "name" => Value::test_string("andres"),
                                    "lang" => Value::test_string("rb"),
                                    "year" => Value::test_string("2019"),
                            })],
                        ),
                    }),
                    "rs" => Value::test_record(record! {
                            "2019" => Value::test_list(
                                vec![Value::test_record(record! {
                                        "name" => Value::test_string("jt"),
                                        "lang" => Value::test_string("rs"),
                                        "year" => Value::test_string("2019"),
                                })],
                            ),
                            "2021" => Value::test_list(
                                vec![Value::test_record(record! {
                                        "name" => Value::test_string("storm"),
                                        "lang" => Value::test_string("rs"),
                                        "year" => Value::test_string("2021"),
                                })],
                            ),
                    }),
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
                item: v.coerce_into_string()?,
                span: name,
            });
            Ok(split(splitter.as_ref(), input, name)?)
        }
        // This uses the same format as the 'requires a column name' error in sort_utils.rs
        None => Err(ShellError::GenericError {
            error: "expected name".into(),
            msg: "requires a column name for splitting".into(),
            span: Some(name),
            help: None,
            inner: vec![],
        }),
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
            let block = move |_, row: &Value| {
                let group_key = if let Value::Record { val: row, .. } = row {
                    row.get(&column_name.item)
                } else {
                    None
                };

                match group_key {
                    Some(group_key) => Ok(group_key.coerce_string()?),
                    None => Err(ShellError::CantFindColumn {
                        col_name: column_name.item.to_string(),
                        span: column_name.span,
                        src_span: row.span(),
                    }),
                }
            };

            data_split(values, Some(&block), span)
        }
        Grouper::ByColumn(None) => {
            let block = move |_, row: &Value| row.coerce_string();

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
            value.coerce_string()
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
    dst_span: Span,
) -> Result<PipelineData, ShellError> {
    let mut splits = indexmap::IndexMap::new();

    match value {
        PipelineData::Value(v, _) => {
            let span = v.span();
            match v {
                Value::Record { val: grouped, .. } => {
                    for (outer_key, list) in grouped.into_owned() {
                        match data_group(&list, splitter, span) {
                            Ok(grouped_vals) => {
                                if let Value::Record { val: sub, .. } = grouped_vals {
                                    for (inner_key, subset) in sub.into_owned() {
                                        let s: &mut IndexMap<String, Value> =
                                            splits.entry(inner_key).or_default();

                                        s.insert(outer_key.clone(), subset.clone());
                                    }
                                }
                            }
                            Err(reason) => return Err(reason),
                        }
                    }
                }
                _ => {
                    return Err(ShellError::OnlySupportsThisInputType {
                        exp_input_type: "Record".into(),
                        wrong_type: v.get_type().to_string(),
                        dst_span,
                        src_span: v.span(),
                    })
                }
            }
        }
        PipelineData::Empty => return Err(ShellError::PipelineEmpty { dst_span }),
        _ => {
            return Err(ShellError::PipelineMismatch {
                exp_input_type: "record".into(),
                dst_span,
                src_span: value.span().unwrap_or(Span::unknown()),
            })
        }
    }

    let record = splits
        .into_iter()
        .map(|(k, rows)| (k, Value::record(rows.into_iter().collect(), dst_span)))
        .collect();

    Ok(PipelineData::Value(Value::record(record, dst_span), None))
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
