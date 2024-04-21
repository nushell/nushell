use nu_engine::command_prelude::*;

use regex::Regex;

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "split column"
    }

    fn signature(&self) -> Signature {
        Signature::build("split column")
            .input_output_types(vec![
                (Type::String, Type::table()),
                (
                    // TODO: no test coverage (is this behavior a bug or a feature?)
                    Type::List(Box::new(Type::String)),
                    Type::table(),
                ),
            ])
            .required(
                "separator",
                SyntaxShape::String,
                "The character or string that denotes what separates columns.",
            )
            .switch("collapse-empty", "remove empty columns", Some('c'))
            .switch("regex", "separator is a regular expression", Some('r'))
            .rest(
                "rest",
                SyntaxShape::String,
                "Column names to give the new columns.",
            )
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Split a string into multiple columns using a separator."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["separate", "divide", "regex"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        split_column(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Split a string into columns by the specified separator",
                example: "'a--b--c' | split column '--'",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                        "column1" => Value::test_string("a"),
                        "column2" => Value::test_string("b"),
                        "column3" => Value::test_string("c"),
                })])),
            },
            Example {
                description: "Split a string into columns of char and remove the empty columns",
                example: "'abc' | split column --collapse-empty ''",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                        "column1" => Value::test_string("a"),
                        "column2" => Value::test_string("b"),
                        "column3" => Value::test_string("c"),
                })])),
            },
            Example {
                description: "Split a list of strings into a table",
                example: "['a-b' 'c-d'] | split column -",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "column1" => Value::test_string("a"),
                        "column2" => Value::test_string("b"),
                    }),
                    Value::test_record(record! {
                        "column1" => Value::test_string("c"),
                        "column2" => Value::test_string("d"),
                    }),
                ])),
            },
            Example {
                description: "Split a list of strings into a table, ignoring padding",
                example: r"['a -  b' 'c  -    d'] | split column --regex '\s*-\s*'",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "column1" => Value::test_string("a"),
                        "column2" => Value::test_string("b"),
                    }),
                    Value::test_record(record! {
                        "column1" => Value::test_string("c"),
                        "column2" => Value::test_string("d"),
                    }),
                ])),
            },
        ]
    }
}

fn split_column(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let name_span = call.head;
    let separator: Spanned<String> = call.req(engine_state, stack, 0)?;
    let rest: Vec<Spanned<String>> = call.rest(engine_state, stack, 1)?;
    let collapse_empty = call.has_flag(engine_state, stack, "collapse-empty")?;

    let regex = if call.has_flag(engine_state, stack, "regex")? {
        Regex::new(&separator.item)
    } else {
        let escaped = regex::escape(&separator.item);
        Regex::new(&escaped)
    }
    .map_err(|e| ShellError::GenericError {
        error: "Error with regular expression".into(),
        msg: e.to_string(),
        span: Some(separator.span),
        help: None,
        inner: vec![],
    })?;

    input.flat_map(
        move |x| split_column_helper(&x, &regex, &rest, collapse_empty, name_span),
        engine_state.ctrlc.clone(),
    )
}

fn split_column_helper(
    v: &Value,
    separator: &Regex,
    rest: &[Spanned<String>],
    collapse_empty: bool,
    head: Span,
) -> Vec<Value> {
    if let Ok(s) = v.coerce_str() {
        let split_result: Vec<_> = separator
            .split(&s)
            .filter(|x| !(collapse_empty && x.is_empty()))
            .collect();
        let positional: Vec<_> = rest.iter().map(|f| f.item.clone()).collect();

        // If they didn't provide column names, make up our own
        let mut record = Record::new();
        if positional.is_empty() {
            let mut gen_columns = vec![];
            for i in 0..split_result.len() {
                gen_columns.push(format!("column{}", i + 1));
            }

            for (&k, v) in split_result.iter().zip(&gen_columns) {
                record.push(v, Value::string(k, head));
            }
        } else {
            for (&k, v) in split_result.iter().zip(&positional) {
                record.push(v, Value::string(k, head));
            }
        }
        vec![Value::record(record, head)]
    } else {
        match v {
            Value::Error { error, .. } => {
                vec![Value::error(*error.clone(), head)]
            }
            v => {
                let span = v.span();
                vec![Value::error(
                    ShellError::PipelineMismatch {
                        exp_input_type: "string".into(),
                        dst_span: head,
                        src_span: span,
                    },
                    span,
                )]
            }
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
