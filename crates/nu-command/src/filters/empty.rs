use nu_engine::{eval_block, CallExt};
use nu_protocol::ast::{Call, CellPath};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct Empty;

impl Command for Empty {
    fn name(&self) -> &str {
        "empty?"
    }

    fn signature(&self) -> Signature {
        Signature::build("empty")
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "the names of the columns to check emptiness",
            )
            .named(
                "block",
                SyntaxShape::Block(Some(vec![SyntaxShape::Any])),
                "an optional block to replace if empty",
                Some('b'),
            )
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Check for empty values."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        empty(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Check if a value is empty",
                example: "'' | empty?",
                result: Some(Value::Bool {
                    val: true,
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "more than one column",
                example: "[[meal size]; [arepa small] [taco '']] | empty? meal size",
                result: Some(
                    Value::List {
                        vals: vec![
                            Value::Record{cols: vec!["meal".to_string(), "size".to_string()], vals: vec![
                                Value::Bool{val: false, span: Span::test_data()},
                                Value::Bool{val: false, span: Span::test_data()}
                            ], span: Span::test_data()},
                            Value::Record{cols: vec!["meal".to_string(), "size".to_string()], vals: vec![
                                Value::Bool{val: false, span: Span::test_data()},
                                Value::Bool{val: true, span: Span::test_data()}
                            ], span: Span::test_data()}
                        ], span: Span::test_data()
                    })
            },
            Example {
                description: "use a block if setting the empty cell contents is wanted",
                example: "[[2020/04/16 2020/07/10 2020/11/16]; ['' [27] [37]]] | empty? 2020/04/16 -b { [33 37] }",
                result: Some(
                    Value::List {
                        vals: vec![
                            Value::Record{
                            cols: vec!["2020/04/16".to_string(), "2020/07/10".to_string(), "2020/11/16".to_string()], 
                            vals: vec![
                                Value::List{vals: vec![
                                    Value::Int{val: 33, span: Span::test_data()},
                                    Value::Int{val: 37, span: Span::test_data()}
                                ], span: Span::test_data()},
                                Value::List{vals: vec![
                                    Value::Int{val: 27, span: Span::test_data()},
                                ], span: Span::test_data()},
                                Value::List{vals: vec![
                                    Value::Int{val: 37, span: Span::test_data()},
                                ], span: Span::test_data()},
                            ], span: Span::test_data()}
                        ], span: Span::test_data()
                    }
                )
            }
        ]
    }
}

fn empty(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
    let head = call.head;
    let columns: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
    let has_block = call.has_flag("block");

    let block = if has_block {
        let block_expr = call
            .get_flag_expr("block")
            .expect("internal error: expected block");

        let block_id = block_expr
            .as_block()
            .ok_or_else(|| ShellError::TypeMismatch("expected row condition".to_owned(), head))?;

        let b = engine_state.get_block(block_id);
        let evaluated_block = eval_block(engine_state, stack, b, PipelineData::new(head))?;
        Some(evaluated_block.into_value(head))
    } else {
        None
    };

    input.map(
        move |value| {
            let columns = columns.clone();

            process_row(value, block.as_ref(), columns, head)
        },
        engine_state.ctrlc.clone(),
    )
}

fn process_row(
    input: Value,
    default_block: Option<&Value>,
    column_paths: Vec<CellPath>,
    head: Span,
) -> Value {
    match input {
        Value::Record {
            cols: _,
            ref vals,
            span,
        } => {
            if column_paths.is_empty() {
                let is_empty = vals.iter().all(|v| v.clone().is_empty());
                if default_block.is_some() {
                    if is_empty {
                        Value::Bool { val: true, span }
                    } else {
                        input.clone()
                    }
                } else {
                    Value::Bool {
                        val: is_empty,
                        span,
                    }
                }
            } else {
                let mut obj = input.clone();
                for column in column_paths {
                    let path = column.into_string();
                    let data = input.get_data_by_key(&path);
                    let is_empty = match data {
                        Some(x) => x.is_empty(),
                        None => true,
                    };

                    let default = if let Some(x) = default_block {
                        if is_empty {
                            x.clone()
                        } else {
                            Value::Bool { val: true, span }
                        }
                    } else {
                        Value::Bool {
                            val: is_empty,
                            span,
                        }
                    };
                    let r = obj.update_cell_path(&column.members, Box::new(move |_| default));
                    if let Err(error) = r {
                        return Value::Error { error };
                    }
                }
                obj
            }
        }
        Value::List { vals, .. } if vals.iter().all(|v| v.as_record().is_ok()) => {
            {
                // we have records
                if column_paths.is_empty() {
                    let is_empty = vals.is_empty() && vals.iter().all(|v| v.clone().is_empty());

                    Value::Bool {
                        val: is_empty,
                        span: head,
                    }
                } else {
                    Value::Bool {
                        val: true,
                        span: head,
                    }
                }
            }
        }
        Value::List { vals, .. } => {
            let empty = vals.iter().all(|v| v.clone().is_empty());
            Value::Bool {
                val: empty,
                span: head,
            }
        }
        other => Value::Bool {
            val: other.is_empty(),
            span: head,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Empty {})
    }
}
