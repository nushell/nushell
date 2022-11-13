use nu_engine::{eval_block, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Closure, Command, EngineState, Stack};
use nu_protocol::{
    Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape,
    Type, Value,
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
            .input_output_types(vec![(
                Type::List(Box::new(Type::Any)),
                Type::Record(vec![]),
            )])
            .optional("grouper", SyntaxShape::Any, "the grouper value to use")
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
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
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
                description: "You can also group by raw values by leaving out the argument",
                example: "['1' '3' '1' '3' '2' '1' '1'] | group-by",
                result: Some(Value::Record {
                    cols: vec!["1".to_string(), "3".to_string(), "2".to_string()],
                    vals: vec![
                        Value::List {
                            vals: vec![
                                Value::test_string("1"),
                                Value::test_string("1"),
                                Value::test_string("1"),
                                Value::test_string("1"),
                            ],
                            span: Span::test_data(),
                        },
                        Value::List {
                            vals: vec![Value::test_string("3"), Value::test_string("3")],
                            span: Span::test_data(),
                        },
                        Value::List {
                            vals: vec![Value::test_string("2")],
                            span: Span::test_data(),
                        },
                    ],
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

enum Grouper {
    ByColumn(Option<Spanned<String>>),
    ByBlock,
}

pub fn group_by(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let name = call.head;

    let grouper: Option<Value> = call.opt(engine_state, stack, 0)?;
    let values: Vec<Value> = input.into_iter().collect();
    let mut keys: Vec<Result<String, ShellError>> = vec![];
    let mut group_strategy = Grouper::ByColumn(None);

    if values.is_empty() {
        return Err(ShellError::GenericError(
            "expected table from pipeline".into(),
            "requires a table input".into(),
            Some(name),
            None,
            Vec::new(),
        ));
    }

    let first = values[0].clone();

    let value_list = Value::List {
        vals: values.clone(),
        span: name,
    };

    match grouper {
        Some(Value::Block { .. }) | Some(Value::Closure { .. }) => {
            let block: Option<Closure> = call.opt(engine_state, stack, 0)?;
            let error_key = "error";

            for value in values {
                if let Some(capture_block) = &block {
                    let mut stack = stack.captures_to_stack(&capture_block.captures);
                    let block = engine_state.get_block(capture_block.block_id);
                    let pipeline = eval_block(
                        engine_state,
                        &mut stack,
                        block,
                        value.into_pipeline_data(),
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
                                    Some(name),
                                    None,
                                    Vec::new(),
                                ));
                            }

                            let value = match collection.get(0) {
                                Some(Value::Error { .. }) | None => Value::String {
                                    val: error_key.to_string(),
                                    span: name,
                                },
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

            group_strategy = Grouper::ByBlock;
        }
        Some(other) => {
            group_strategy = Grouper::ByColumn(Some(Spanned {
                item: other.as_string()?,
                span: name,
            }));
        }
        _ => {}
    }

    let name = if let Ok(span) = first.span() {
        span
    } else {
        name
    };

    let group_value = match group_strategy {
        Grouper::ByBlock => {
            let map = keys;

            let block = Box::new(move |idx: usize, row: &Value| match map.get(idx) {
                Some(Ok(key)) => Ok(key.clone()),
                Some(Err(reason)) => Err(reason.clone()),
                None => row.as_string(),
            });

            data_group(&value_list, &Some(block), name)
        }
        Grouper::ByColumn(column_name) => group(&column_name, &value_list, name),
    };

    Ok(PipelineData::Value(group_value?, None))
}

#[allow(clippy::type_complexity)]
pub fn data_group(
    values: &Value,
    grouper: &Option<Box<dyn Fn(usize, &Value) -> Result<String, ShellError> + Send>>,
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

    let mut cols = vec![];
    let mut vals = vec![];

    for (k, v) in groups {
        cols.push(k.to_string());
        vals.push(Value::List { vals: v, span });
    }

    Ok(Value::Record { cols, vals, span })
}

pub fn group(
    column_name: &Option<Spanned<String>>,
    values: &Value,
    span: Span,
) -> Result<Value, ShellError> {
    let name = span;

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

            data_group(values, &Some(block), name)
        }
        Grouper::ByColumn(None) => {
            let block = Box::new(move |_, row: &Value| row.as_string());

            data_group(values, &Some(block), name)
        }
        Grouper::ByBlock => Err(ShellError::NushellFailed(
            "Block not implemented: This should never happen.".into(),
        )),
    }
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
