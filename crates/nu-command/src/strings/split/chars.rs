use crate::grapheme_flags;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SpannedValue, Type,
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
                result: Some(SpannedValue::List {
                    vals: vec![
                        SpannedValue::test_string("h"),
                        SpannedValue::test_string("e"),
                        SpannedValue::test_string("l"),
                        SpannedValue::test_string("l"),
                        SpannedValue::test_string("o"),
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Split on grapheme clusters",
                example: "'🇯🇵ほげ' | split chars -g",
                result: Some(SpannedValue::List {
                    vals: vec![
                        SpannedValue::test_string("🇯🇵"),
                        SpannedValue::test_string("ほ"),
                        SpannedValue::test_string("げ"),
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Split multiple strings into lists of characters",
                example: "['hello', 'world'] | split chars",
                result: Some(SpannedValue::test_list(vec![
                    SpannedValue::test_list(vec![
                        SpannedValue::test_string("h"),
                        SpannedValue::test_string("e"),
                        SpannedValue::test_string("l"),
                        SpannedValue::test_string("l"),
                        SpannedValue::test_string("o"),
                    ]),
                    SpannedValue::test_list(vec![
                        SpannedValue::test_string("w"),
                        SpannedValue::test_string("o"),
                        SpannedValue::test_string("r"),
                        SpannedValue::test_string("l"),
                        SpannedValue::test_string("d"),
                    ]),
                ])),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        split_chars(engine_state, call, input)
    }
}

fn split_chars(
    engine_state: &EngineState,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;

    let graphemes = grapheme_flags(call)?;
    input.map(
        move |x| split_chars_helper(&x, span, graphemes),
        engine_state.ctrlc.clone(),
    )
}

fn split_chars_helper(v: &SpannedValue, name: Span, graphemes: bool) -> SpannedValue {
    match v.span() {
        Ok(v_span) => {
            if let Ok(s) = v.as_string() {
                SpannedValue::List {
                    vals: if graphemes {
                        s.graphemes(true)
                            .collect::<Vec<_>>()
                            .into_iter()
                            .map(move |x| SpannedValue::string(x, v_span))
                            .collect()
                    } else {
                        s.chars()
                            .collect::<Vec<_>>()
                            .into_iter()
                            .map(move |x| SpannedValue::string(x, v_span))
                            .collect()
                    },
                    span: v_span,
                }
            } else {
                SpannedValue::Error {
                    error: Box::new(ShellError::PipelineMismatch {
                        exp_input_type: "string".into(),
                        dst_span: name,
                        src_span: v_span,
                    }),
                }
            }
        }
        Err(error) => SpannedValue::Error {
            error: Box::new(error),
        },
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
