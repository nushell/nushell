use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Type,
    Value,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "split column"
    }

    fn signature(&self) -> Signature {
        Signature::build("split column")
            .input_output_types(vec![
                (Type::String, Type::Table(vec![])),
                (
                    // TODO: no test coverage (is this behavior a bug or a feature?)
                    Type::List(Box::new(Type::String)),
                    Type::Table(vec![]),
                ),
            ])
            .required(
                "separator",
                SyntaxShape::String,
                "the character or string that denotes what separates columns",
            )
            .switch("collapse-empty", "remove empty columns", Some('c'))
            .rest(
                "rest",
                SyntaxShape::String,
                "column names to give the new columns",
            )
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Split a string into multiple columns using a separator"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["separate", "divide"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        split_column(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Split a string into columns by the specified separator",
                example: "echo 'a--b--c' | split column '--'",
                result: Some(Value::List {
                    vals: vec![Value::Record {
                        cols: vec![
                            "column1".to_string(),
                            "column2".to_string(),
                            "column3".to_string(),
                        ],
                        vals: vec![
                            Value::test_string("a"),
                            Value::test_string("b"),
                            Value::test_string("c"),
                        ],
                        span: Span::test_data(),
                    }],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Split a string into columns of char and remove the empty columns",
                example: "'abc' | split column -c ''",
                result: Some(Value::List {
                    vals: vec![Value::Record {
                        cols: vec![
                            "column1".to_string(),
                            "column2".to_string(),
                            "column3".to_string(),
                        ],
                        vals: vec![
                            Value::test_string("a"),
                            Value::test_string("b"),
                            Value::test_string("c"),
                        ],
                        span: Span::test_data(),
                    }],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Split a list of strings into a table",
                example: "['a-b' 'c-d'] | split column -",
                result: Some(Value::List {
                    vals: vec![
                        Value::Record {
                            cols: vec!["column1".to_string(), "column2".to_string()],
                            vals: vec![Value::test_string("a"), Value::test_string("b")],
                            span: Span::test_data(),
                        },
                        Value::Record {
                            cols: vec!["column1".to_string(), "column2".to_string()],
                            vals: vec![Value::test_string("c"), Value::test_string("d")],
                            span: Span::test_data(),
                        },
                    ],
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

fn split_column(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
    let name_span = call.head;
    let separator: Spanned<String> = call.req(engine_state, stack, 0)?;
    let rest: Vec<Spanned<String>> = call.rest(engine_state, stack, 1)?;
    let collapse_empty = call.has_flag("collapse-empty");

    input.flat_map(
        move |x| split_column_helper(&x, &separator, &rest, collapse_empty, name_span),
        engine_state.ctrlc.clone(),
    )
}

fn split_column_helper(
    v: &Value,
    separator: &Spanned<String>,
    rest: &[Spanned<String>],
    collapse_empty: bool,
    head: Span,
) -> Vec<Value> {
    if let Ok(s) = v.as_string() {
        let split_result: Vec<_> = if collapse_empty {
            s.split(&separator.item).filter(|s| !s.is_empty()).collect()
        } else {
            s.split(&separator.item).collect()
        };

        let positional: Vec<_> = rest.iter().map(|f| f.item.clone()).collect();

        // If they didn't provide column names, make up our own

        let mut cols = vec![];
        let mut vals = vec![];
        if positional.is_empty() {
            let mut gen_columns = vec![];
            for i in 0..split_result.len() {
                gen_columns.push(format!("column{}", i + 1));
            }

            for (&k, v) in split_result.iter().zip(&gen_columns) {
                cols.push(v.to_string());
                vals.push(Value::string(k, head));
            }
        } else {
            for (&k, v) in split_result.iter().zip(&positional) {
                cols.push(v.into());
                vals.push(Value::string(k, head));
            }
        }
        vec![Value::Record {
            cols,
            vals,
            span: head,
        }]
    } else {
        match v.span() {
            Ok(span) => vec![Value::Error {
                error: ShellError::PipelineMismatch("string".into(), head, span),
            }],
            Err(error) => vec![Value::Error { error }],
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }
}
