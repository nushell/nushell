use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
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
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        rename(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Rename a column",
                example: "[[a, b]; [1, 2]] | rename my_column",
                result: Some(Value::List {
                    vals: vec![Value::Record {
                        cols: vec!["my_column".to_string(), "b".to_string()],
                        vals: vec![Value::test_int(1), Value::test_int(2)],
                        span: Span::test_data(),
                    }],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Rename many columns",
                example: "[[a, b, c]; [1, 2, 3]] | rename eggs ham bacon",
                result: Some(Value::List {
                    vals: vec![Value::Record {
                        cols: vec!["eggs".to_string(), "ham".to_string(), "bacon".to_string()],
                        vals: vec![Value::test_int(1), Value::test_int(2), Value::test_int(3)],
                        span: Span::test_data(),
                    }],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Rename a specific column",
                example: "[[a, b, c]; [1, 2, 3]] | rename -c [a ham]",
                result: Some(Value::List {
                    vals: vec![Value::Record {
                        cols: vec!["ham".to_string(), "b".to_string(), "c".to_string()],
                        vals: vec![Value::test_int(1), Value::test_int(2), Value::test_int(3)],
                        span: Span::test_data(),
                    }],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Rename the fields of a record",
                example: "{a: 1 b: 2} | rename x y",
                result: Some(Value::Record {
                    cols: vec!["x".to_string(), "y".to_string()],
                    vals: vec![Value::test_int(1), Value::test_int(2)],
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

fn rename(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
    let specified_column: Option<Vec<String>> = call.get_flag(engine_state, stack, "column")?;
    // get the span for the column's name to be changed and for the given list
    let (specified_col_span, list_span) = if let Some(Value::List {
        vals: columns,
        span: column_span,
    }) = call.get_flag(engine_state, stack, "column")?
    {
        (Some(columns[0].span()?), column_span)
    } else {
        (None, call.head)
    };

    if let Some(ref cols) = specified_column {
        if cols.len() != 2 {
            return Err(ShellError::UnsupportedInput(
                    "The list must contain only two values: the column's name and its replacement value"
                        .to_string(),
                        list_span,
                ));
        }
    }

    let columns: Vec<String> = call.rest(engine_state, stack, 0)?;
    let metadata = input.metadata();

    let head_span = call.head;
    input
        .map(
            move |item| match item {
                Value::Record {
                    mut cols,
                    vals,
                    span,
                } => {
                    match &specified_column {
                        Some(c) => {
                            // check if the specified column to be renamed exists
                            if !cols.contains(&c[0]) {
                                return Value::Error {
                                    error: ShellError::UnsupportedInput(
                                        "The specified column does not exist".to_string(),
                                        specified_col_span.unwrap_or(span),
                                    ),
                                };
                            }
                            for (idx, val) in cols.iter_mut().enumerate() {
                                if *val == c[0] {
                                    cols[idx] = c[1].to_string();
                                    break;
                                }
                            }
                        }
                        None => {
                            for (idx, val) in columns.iter().enumerate() {
                                if idx >= cols.len() {
                                    // skip extra new columns names if we already reached the final column
                                    break;
                                }
                                cols[idx] = val.clone();
                            }
                        }
                    }

                    Value::Record { cols, vals, span }
                }
                x => Value::Error {
                    error: ShellError::UnsupportedInput(
                        "can't rename: input is not table, so no column names available for rename"
                            .to_string(),
                        x.span().unwrap_or(head_span),
                    ),
                },
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
