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
        "split row"
    }

    fn signature(&self) -> Signature {
        Signature::build("split row")
            .input_output_types(vec![(Type::String, Type::List(Box::new(Type::String)))])
            .vectorizes_over_list(true)
            .required(
                "separator",
                SyntaxShape::String,
                "the character that denotes what separates rows",
            )
            .named(
                "number",
                SyntaxShape::Int,
                "Split into maximum number of items",
                Some('n'),
            )
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Split a string into multiple rows using a separator"
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
        split_row(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Split a string into rows of char",
                example: "echo 'abc' | split row ''",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_string("a"),
                        Value::test_string("b"),
                        Value::test_string("c"),
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Split a string into rows by the specified separator",
                example: "echo 'a--b--c' | split row '--'",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_string("a"),
                        Value::test_string("b"),
                        Value::test_string("c"),
                    ],
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

fn split_row(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
    let name_span = call.head;
    let separator: Spanned<String> = call.req(engine_state, stack, 0)?;
    let max_split: Option<usize> = call.get_flag(engine_state, stack, "number")?;
    input.flat_map(
        move |x| split_row_helper(&x, &separator, max_split, name_span),
        engine_state.ctrlc.clone(),
    )
}

fn split_row_helper(
    v: &Value,
    separator: &Spanned<String>,
    max_split: Option<usize>,
    name: Span,
) -> Vec<Value> {
    match v.span() {
        Ok(v_span) => {
            if let Ok(s) = v.as_string() {
                match max_split {
                    Some(max_split) => s
                        .splitn(max_split, &separator.item)
                        .filter_map(|s| {
                            if s.trim() != "" {
                                Some(Value::string(s, v_span))
                            } else {
                                None
                            }
                        })
                        .collect(),
                    None => s
                        .split(&separator.item)
                        .filter_map(|s| {
                            if s.trim() != "" {
                                Some(Value::string(s, v_span))
                            } else {
                                None
                            }
                        })
                        .collect(),
                }
            } else {
                vec![Value::Error {
                    error: ShellError::PipelineMismatch("string".into(), name, v_span),
                }]
            }
        }
        Err(error) => vec![Value::Error { error }],
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
