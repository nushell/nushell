use nu_engine::{eval_block_with_early_return, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Closure, Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, Record, ShellError, Signature, Span,
    SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct Rename;

impl Command for Rename {
    fn name(&self) -> &str {
        "rename"
    }

    fn signature(&self) -> Signature {
        Signature::build("rename")
            .input_output_types(vec![
                (Type::Record(vec![]), Type::Record(vec![])),
                (Type::Table(vec![]), Type::Table(vec![])),
            ])
            .named(
                "column",
                SyntaxShape::List(Box::new(SyntaxShape::String)),
                "column name to be changed",
                Some('c'),
            )
            .named(
                "block",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                "A closure to apply changes on each column",
                Some('b'),
            )
            .rest("rest", SyntaxShape::String, "the new names for the columns")
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Creates a new table with columns renamed."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        rename(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Rename a column",
                example: "[[a, b]; [1, 2]] | rename my_column",
                result: Some(Value::list(
                    vec![Value::test_record(Record {
                        cols: vec!["my_column".to_string(), "b".to_string()],
                        vals: vec![Value::test_int(1), Value::test_int(2)],
                    })],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Rename many columns",
                example: "[[a, b, c]; [1, 2, 3]] | rename eggs ham bacon",
                result: Some(Value::list(
                    vec![Value::test_record(Record {
                        cols: vec!["eggs".to_string(), "ham".to_string(), "bacon".to_string()],
                        vals: vec![Value::test_int(1), Value::test_int(2), Value::test_int(3)],
                    })],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Rename a specific column",
                example: "[[a, b, c]; [1, 2, 3]] | rename -c [a ham]",
                result: Some(Value::list(
                    vec![Value::test_record(Record {
                        cols: vec!["ham".to_string(), "b".to_string(), "c".to_string()],
                        vals: vec![Value::test_int(1), Value::test_int(2), Value::test_int(3)],
                    })],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Rename the fields of a record",
                example: "{a: 1 b: 2} | rename x y",
                result: Some(Value::test_record(Record {
                    cols: vec!["x".to_string(), "y".to_string()],
                    vals: vec![Value::test_int(1), Value::test_int(2)],
                })),
            },
            Example {
                description: "Rename fields based on a given closure",
                example: "{abc: 1, bbc: 2} | rename -b {str replace -a 'b' 'z'}",
                result: Some(Value::test_record(Record {
                    cols: vec!["azc".to_string(), "zzc".to_string()],
                    vals: vec![Value::test_int(1), Value::test_int(2)],
                })),
            },
        ]
    }
}

fn rename(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let specified_column: Option<Vec<String>> = call.get_flag(engine_state, stack, "column")?;
    // get the span for the column's name to be changed and for the given list
    let column_flag: Option<Value> = call.get_flag(engine_state, stack, "column")?;
    let (specified_col_span, list_span) = match column_flag {
        Some(column_flag) => {
            let column_span = column_flag.span();
            match column_flag {
                Value::List { vals: columns, .. } => {
                    if columns.is_empty() {
                        return Err(ShellError::TypeMismatch { err_message: "The column list cannot be empty and must contain only two values: the column's name and its replacement value"
                        .to_string(), span: column_span });
                    } else {
                        (Some(columns[0].span()), column_span)
                    }
                }
                _ => (None, call.head),
            }
        }
        None => (None, call.head),
    };

    if let Some(ref cols) = specified_column {
        if cols.len() != 2 {
            return Err(ShellError::TypeMismatch { err_message: "The column list must contain only two values: the column's name and its replacement value"
                        .to_string(), span: list_span });
        }
    }

    let redirect_stdout = call.redirect_stdout;
    let redirect_stderr = call.redirect_stderr;
    let block_info =
        if let Some(capture_block) = call.get_flag::<Closure>(engine_state, stack, "block")? {
            let engine_state = engine_state.clone();
            let block = engine_state.get_block(capture_block.block_id).clone();
            let stack = stack.captures_to_stack(&capture_block.captures);
            let orig_env_vars = stack.env_vars.clone();
            let orig_env_hidden = stack.env_hidden.clone();
            Some((engine_state, block, stack, orig_env_vars, orig_env_hidden))
        } else {
            None
        };

    let columns: Vec<String> = call.rest(engine_state, stack, 0)?;
    let metadata = input.metadata();

    let head_span = call.head;
    input
        .map(
            move |item| {
                let span = item.span();
                match item {
                    Value::Record {
                        val: mut record, ..
                    } => {
                        if let Some((engine_state, block, mut stack, env_vars, env_hidden)) =
                            block_info.clone()
                        {
                            for c in &mut record.cols {
                                stack.with_env(&env_vars, &env_hidden);

                                if let Some(var) = block.signature.get_positional(0) {
                                    if let Some(var_id) = &var.var_id {
                                        stack.add_var(*var_id, Value::string(c.clone(), span))
                                    }
                                }
                                let eval_result = eval_block_with_early_return(
                                    &engine_state,
                                    &mut stack,
                                    &block,
                                    Value::string(c.clone(), span).into_pipeline_data(),
                                    redirect_stdout,
                                    redirect_stderr,
                                );
                                match eval_result {
                                    Err(e) => return Value::error(e, span),
                                    Ok(res) => match res.collect_string_strict(span) {
                                        Err(e) => return Value::error(e, span),
                                        Ok(new_c) => *c = new_c.0,
                                    },
                                }
                            }
                        } else {
                            match &specified_column {
                                Some(c) => {
                                    // check if the specified column to be renamed exists
                                    if !record.cols.contains(&c[0]) {
                                        return Value::error(
                                            ShellError::UnsupportedInput(
                                                format!(
                                                    "The column '{}' does not exist in the input",
                                                    &c[0]
                                                ),
                                                "value originated from here".into(),
                                                // Arrow 1 points at the specified column name,
                                                specified_col_span.unwrap_or(head_span),
                                                // Arrow 2 points at the input value.
                                                span,
                                            ),
                                            span,
                                        );
                                    }
                                    for (idx, val) in record.cols.iter_mut().enumerate() {
                                        if *val == c[0] {
                                            record.cols[idx] = c[1].to_string();
                                            break;
                                        }
                                    }
                                }
                                None => {
                                    for (idx, val) in columns.iter().enumerate() {
                                        if idx >= record.len() {
                                            // skip extra new columns names if we already reached the final column
                                            break;
                                        }
                                        record.cols[idx] = val.clone();
                                    }
                                }
                            }
                        }

                        Value::record(record, span)
                    }
                    // Propagate errors by explicitly matching them before the final case.
                    Value::Error { .. } => item.clone(),
                    other => Value::error(
                        ShellError::OnlySupportsThisInputType {
                            exp_input_type: "record".into(),
                            wrong_type: other.get_type().to_string(),
                            dst_span: head_span,
                            src_span: other.span(),
                        },
                        head_span,
                    ),
                }
            },
            engine_state.ctrlc.clone(),
        )
        .map(|x| x.set_metadata(metadata))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Rename {})
    }
}
