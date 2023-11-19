use nu_engine::{eval_block, CallExt};
use nu_protocol::ast::{Call, CellPath};
use nu_protocol::engine::{Closure, Command, EngineState, Stack};
use nu_protocol::{
    record, Category, Example, IntoPipelineData, PipelineData, Record, ShellError, Signature, Span,
    SyntaxShape, Type, Value,
};

use indexmap::IndexMap;

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
            .input_output_types(vec![
                (Type::List(Box::new(Type::Any)), Type::Record(vec![])),
                (Type::List(Box::new(Type::Any)), Type::Table(vec![])),
            ])
            .switch(
                "to-table",
                "Return a table with \"groups\" and \"items\" columns",
                None,
            )
            .optional(
                "grouper",
                SyntaxShape::OneOf(vec![
                    SyntaxShape::CellPath,
                    SyntaxShape::Block,
                    SyntaxShape::Closure(None),
                    SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                ]),
                "the path to the column to group on",
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
                    "txt" =>  Value::test_list(
                            vec![
                                Value::test_string("foo.txt"),
                                Value::test_string("baz.txt"),
                            ],
                        ),
                    "csv" => Value::test_list(
                            vec![Value::test_string("bar.csv")],
                        ),
                })),
            },
            Example {
                description: "You can also group by raw values by leaving out the argument",
                example: "['1' '3' '1' '3' '2' '1' '1'] | group-by",
                result: Some(Value::test_record(record! {
                    "1" =>  Value::test_list(
                            vec![
                                Value::test_string("1"),
                                Value::test_string("1"),
                                Value::test_string("1"),
                                Value::test_string("1"),
                            ],
                        ),
                    "3" =>  Value::test_list(
                            vec![Value::test_string("3"), Value::test_string("3")],
                        ),
                    "2" => Value::test_list(
                            vec![Value::test_string("2")],
                        ),
                })),
            },
            Example {
                description: "You can also output a table instead of a record",
                example: "['1' '3' '1' '3' '2' '1' '1'] | group-by --to-table",
                result: Some(Value::test_list(vec![
                    Value::test_record(
                        record! {
                            "group" => Value::test_string("1"),
                            "items" => Value::test_list(
                                vec![
                                    Value::test_string("1"),
                                    Value::test_string("1"),
                                    Value::test_string("1"),
                                    Value::test_string("1"),
                                ]
                            )
                        }
                    ),
                    Value::test_record(
                        record! {
                            "group" => Value::test_string("3"),
                            "items" => Value::test_list(
                                vec![
                                    Value::test_string("3"),
                                    Value::test_string("3"),
                                ]
                            )
                        }
                    ),
                    Value::test_record(
                        record! {
                            "group" => Value::test_string("2"),
                            "items" => Value::test_list(
                                vec![
                                    Value::test_string("2"),
                                ]
                            )
                        }
                    ),
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
    let span = call.head;

    let grouper: Option<Value> = call.opt(engine_state, stack, 0)?;
    let values: Vec<Value> = input.into_iter().collect();

    if values.is_empty() {
        return Ok(PipelineData::Value(
            Value::record(Record::new(), Span::unknown()),
            None,
        ));
    }

    let groups = match grouper {
        Some(v) => {
            let span = v.span();
            match v {
                Value::CellPath { val, .. } => group_cell_path(val, values)?,
                Value::Block { .. } | Value::Closure { .. } => {
                    let block: Option<Closure> = call.opt(engine_state, stack, 0)?;
                    group_closure(&values, span, block, stack, engine_state, call)?
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

    let value = if call.has_flag("to-table") {
        groups_to_table(groups, span)
    } else {
        groups_to_record(groups, span)
    };

    Ok(PipelineData::Value(value, None))
}

pub fn group_cell_path(
    column_name: CellPath,
    values: Vec<Value>,
) -> Result<IndexMap<String, Vec<Value>>, ShellError> {
    let mut groups: IndexMap<String, Vec<Value>> = IndexMap::new();

    for value in values.into_iter() {
        let group_key = value
            .clone()
            .follow_cell_path(&column_name.members, false)?;
        if matches!(group_key, Value::Nothing { .. }) {
            continue; // likely the result of a failed optional access, ignore this value
        }

        let group_key = group_key.as_string()?;
        let group = groups.entry(group_key).or_default();
        group.push(value);
    }

    Ok(groups)
}

pub fn group_no_grouper(values: Vec<Value>) -> Result<IndexMap<String, Vec<Value>>, ShellError> {
    let mut groups: IndexMap<String, Vec<Value>> = IndexMap::new();

    for value in values.into_iter() {
        let group_key = value.as_string()?;
        let group = groups.entry(group_key).or_default();
        group.push(value);
    }

    Ok(groups)
}

// TODO: refactor this, it's a bit of a mess
fn group_closure(
    values: &[Value],
    span: Span,
    block: Option<Closure>,
    stack: &mut Stack,
    engine_state: &EngineState,
    call: &Call,
) -> Result<IndexMap<String, Vec<Value>>, ShellError> {
    let error_key = "error";
    let mut keys: Vec<Result<String, ShellError>> = vec![];
    let value_list = Value::list(values.to_vec(), span);

    for value in values {
        if let Some(capture_block) = &block {
            let mut stack = stack.captures_to_stack(capture_block.captures.clone());
            let block = engine_state.get_block(capture_block.block_id);
            let pipeline = eval_block(
                engine_state,
                &mut stack,
                block,
                value.clone().into_pipeline_data(),
                call.redirect_stdout,
                call.redirect_stderr,
            );

            match pipeline {
                Ok(s) => {
                    let collection: Vec<Value> = s.into_iter().collect();

                    if collection.len() > 1 {
                        return Err(ShellError::GenericError(
                            "expected one value from the block".into(),
                            "requires a table with one value for grouping".into(),
                            Some(span),
                            None,
                            Vec::new(),
                        ));
                    }

                    let value = match collection.first() {
                        Some(Value::Error { .. }) | None => Value::string(error_key, span),
                        Some(return_value) => return_value.clone(),
                    };

                    keys.push(value.as_string());
                }
                Err(_) => {
                    keys.push(Ok(error_key.into()));
                }
            }
        }
    }
    let map = keys;
    let block = Box::new(move |idx: usize, row: &Value| match map.get(idx) {
        Some(Ok(key)) => Ok(key.clone()),
        Some(Err(reason)) => Err(reason.clone()),
        None => row.as_string(),
    });

    let grouper = &Some(block);
    let mut groups: IndexMap<String, Vec<Value>> = IndexMap::new();

    for (idx, value) in value_list.into_pipeline_data().into_iter().enumerate() {
        let group_key = if let Some(ref grouper) = grouper {
            grouper(idx, &value)
        } else {
            value.as_string()
        };

        let group = groups.entry(group_key?).or_default();
        group.push(value);
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
