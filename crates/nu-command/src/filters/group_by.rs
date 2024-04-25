use indexmap::IndexMap;
use nu_engine::{command_prelude::*, ClosureEval};
use nu_protocol::engine::Closure;

#[derive(Clone)]
pub struct GroupBy;

impl Command for GroupBy {
    fn name(&self) -> &str {
        "group-by"
    }

    fn signature(&self) -> Signature {
        Signature::build("group-by")
            // TODO: It accepts Table also, but currently there is no Table
            // example. Perhaps Table should be a subtype of List, in which case
            // the current signature would suffice even when a Table example
            // exists.
            .input_output_types(vec![(Type::List(Box::new(Type::Any)), Type::Any)])
            .switch(
                "to-table",
                "Return a table with \"groups\" and \"items\" columns",
                None,
            )
            .optional(
                "grouper",
                SyntaxShape::OneOf(vec![
                    SyntaxShape::CellPath,
                    SyntaxShape::Closure(None),
                    SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                ]),
                "The path to the column to group on.",
            )
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Splits a list or table into groups, and returns a record containing those groups."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        group_by(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Group items by the \"type\" column's values",
                example: r#"ls | group-by type"#,
                result: None,
            },
            Example {
                description: "Group items by the \"foo\" column's values, ignoring records without a \"foo\" column",
                example: r#"open cool.json | group-by foo?"#,
                result: None,
            },
            Example {
                description: "Group using a block which is evaluated against each input value",
                example: "[foo.txt bar.csv baz.txt] | group-by { path parse | get extension }",
                result: Some(Value::test_record(record! {
                    "txt" => Value::test_list(vec![
                        Value::test_string("foo.txt"),
                        Value::test_string("baz.txt"),
                    ]),
                    "csv" => Value::test_list(vec![Value::test_string("bar.csv")]),
                })),
            },
            Example {
                description: "You can also group by raw values by leaving out the argument",
                example: "['1' '3' '1' '3' '2' '1' '1'] | group-by",
                result: Some(Value::test_record(record! {
                    "1" => Value::test_list(vec![
                        Value::test_string("1"),
                        Value::test_string("1"),
                        Value::test_string("1"),
                        Value::test_string("1"),
                    ]),
                    "3" => Value::test_list(vec![
                        Value::test_string("3"),
                        Value::test_string("3"),
                    ]),
                    "2" => Value::test_list(vec![Value::test_string("2")]),
                })),
            },
            Example {
                description: "You can also output a table instead of a record",
                example: "['1' '3' '1' '3' '2' '1' '1'] | group-by --to-table",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "group" => Value::test_string("1"),
                        "items" => Value::test_list(vec![
                            Value::test_string("1"),
                            Value::test_string("1"),
                            Value::test_string("1"),
                            Value::test_string("1"),
                        ]),
                    }),
                    Value::test_record(record! {
                        "group" => Value::test_string("3"),
                        "items" => Value::test_list(vec![
                            Value::test_string("3"),
                            Value::test_string("3"),
                        ]),
                    }),
                    Value::test_record(record! {
                        "group" => Value::test_string("2"),
                        "items" => Value::test_list(vec![Value::test_string("2")]),
                    }),
                ])),
            },
        ]
    }
}

pub fn group_by(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let head = call.head;
    let grouper: Option<Value> = call.opt(engine_state, stack, 0)?;
    let to_table = call.has_flag(engine_state, stack, "to-table")?;

    let values: Vec<Value> = input.into_iter().collect();
    if values.is_empty() {
        return Ok(Value::record(Record::new(), head).into_pipeline_data());
    }

    let groups = match grouper {
        Some(grouper) => {
            let span = grouper.span();
            match grouper {
                Value::CellPath { val, .. } => group_cell_path(val, values)?,
                Value::Closure { val, .. } => {
                    group_closure(values, span, val, engine_state, stack)?
                }
                _ => {
                    return Err(ShellError::TypeMismatch {
                        err_message: "unsupported grouper type".to_string(),
                        span,
                    })
                }
            }
        }
        None => group_no_grouper(values)?,
    };

    let value = if to_table {
        groups_to_table(groups, head)
    } else {
        groups_to_record(groups, head)
    };

    Ok(value.into_pipeline_data())
}

fn group_cell_path(
    column_name: CellPath,
    values: Vec<Value>,
) -> Result<IndexMap<String, Vec<Value>>, ShellError> {
    let mut groups = IndexMap::<_, Vec<_>>::new();

    for value in values.into_iter() {
        let key = value
            .clone()
            .follow_cell_path(&column_name.members, false)?;

        if matches!(key, Value::Nothing { .. }) {
            continue; // likely the result of a failed optional access, ignore this value
        }

        let key = key.coerce_string()?;
        groups.entry(key).or_default().push(value);
    }

    Ok(groups)
}

fn group_no_grouper(values: Vec<Value>) -> Result<IndexMap<String, Vec<Value>>, ShellError> {
    let mut groups = IndexMap::<_, Vec<_>>::new();

    for value in values.into_iter() {
        let key = value.coerce_string()?;
        groups.entry(key).or_default().push(value);
    }

    Ok(groups)
}

fn group_closure(
    values: Vec<Value>,
    span: Span,
    closure: Closure,
    engine_state: &EngineState,
    stack: &mut Stack,
) -> Result<IndexMap<String, Vec<Value>>, ShellError> {
    let mut groups = IndexMap::<_, Vec<_>>::new();
    let mut closure = ClosureEval::new(engine_state, stack, closure);

    for value in values {
        let key = closure
            .run_with_value(value.clone())?
            .into_value(span)
            .coerce_into_string()?;

        groups.entry(key).or_default().push(value);
    }

    Ok(groups)
}

fn groups_to_record(groups: IndexMap<String, Vec<Value>>, span: Span) -> Value {
    Value::record(
        groups
            .into_iter()
            .map(|(k, v)| (k, Value::list(v, span)))
            .collect(),
        span,
    )
}

fn groups_to_table(groups: IndexMap<String, Vec<Value>>, span: Span) -> Value {
    Value::list(
        groups
            .into_iter()
            .map(|(group, items)| {
                Value::record(
                    record! {
                        "group" => Value::string(group, span),
                        "items" => Value::list(items, span),
                    },
                    span,
                )
            })
            .collect(),
        span,
    )
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(GroupBy {})
    }
}
