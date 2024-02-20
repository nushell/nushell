use crate::grapheme_flags;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Type, Value,
};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "split chars"
    }

    fn signature(&self) -> Signature {
        Signature::build("split chars")
            .input_output_types(vec![
                (Type::String, Type::List(Box::new(Type::String))),
                (
                    Type::List(Box::new(Type::String)),
                    Type::List(Box::new(Type::List(Box::new(Type::String)))),
                ),
            ])
            .allow_variants_without_examples(true)
            .switch("grapheme-clusters", "split on grapheme clusters", Some('g'))
            .switch(
                "code-points",
                "split on code points (default; splits combined characters)",
                Some('c'),
            )
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Split a string into a list of characters."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["character", "separate", "divide"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Split the string into a list of characters",
                example: "'hello' | split chars",
                result: Some(Value::list(
                    vec![
                        Value::test_string("h"),
                        Value::test_string("e"),
                        Value::test_string("l"),
                        Value::test_string("l"),
                        Value::test_string("o"),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Split on grapheme clusters",
                example: "'🇯🇵ほげ' | split chars --grapheme-clusters",
                result: Some(Value::list(
                    vec![
                        Value::test_string("🇯🇵"),
                        Value::test_string("ほ"),
                        Value::test_string("げ"),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Split multiple strings into lists of characters",
                example: "['hello', 'world'] | split chars",
                result: Some(Value::test_list(vec![
                    Value::test_list(vec![
                        Value::test_string("h"),
                        Value::test_string("e"),
                        Value::test_string("l"),
                        Value::test_string("l"),
                        Value::test_string("o"),
                    ]),
                    Value::test_list(vec![
                        Value::test_string("w"),
                        Value::test_string("o"),
                        Value::test_string("r"),
                        Value::test_string("l"),
                        Value::test_string("d"),
                    ]),
                ])),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        split_chars(engine_state, stack, call, input)
    }
}

fn split_chars(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;

    let graphemes = grapheme_flags(engine_state, stack, call)?;
    input.map(
        move |x| split_chars_helper(&x, span, graphemes),
        engine_state.ctrlc.clone(),
    )
}

fn split_chars_helper(v: &Value, name: Span, graphemes: bool) -> Value {
    let span = v.span();
    match v {
        Value::Error { error, .. } => Value::error(*error.clone(), span),
        v => {
            let v_span = v.span();
            if let Ok(s) = v.coerce_str() {
                Value::list(
                    if graphemes {
                        s.graphemes(true)
                            .collect::<Vec<_>>()
                            .into_iter()
                            .map(move |x| Value::string(x, v_span))
                            .collect()
                    } else {
                        s.chars()
                            .collect::<Vec<_>>()
                            .into_iter()
                            .map(move |x| Value::string(x, v_span))
                            .collect()
                    },
                    v_span,
                )
            } else {
                Value::error(
                    ShellError::PipelineMismatch {
                        exp_input_type: "string".into(),
                        dst_span: name,
                        src_span: v_span,
                    },
                    name,
                )
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
