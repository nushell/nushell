use nu_cmd_base::input_handler::{CellPathOnlyArgs, operate};
use nu_engine::command_prelude::*;

use percent_encoding::percent_decode_str;

#[derive(Clone)]
pub struct UrlDecode;

impl Command for UrlDecode {
    fn name(&self) -> &str {
        "url decode"
    }

    fn signature(&self) -> Signature {
        Signature::build("url decode")
            .input_output_types(vec![
                (Type::String, Type::String),
                (
                    Type::List(Box::new(Type::String)),
                    Type::List(Box::new(Type::String)),
                ),
                (Type::table(), Type::table()),
                (Type::record(), Type::record()),
            ])
            .allow_variants_without_examples(true)
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, url decode strings at the given cell paths.",
            )
            .category(Category::Strings)
    }

    fn description(&self) -> &str {
        "Converts a percent-encoded web safe string to a string."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["string", "text", "convert"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
        let args = CellPathOnlyArgs::from(cell_paths);
        operate(action, args, input, call.head, engine_state.signals())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Decode a url with escape characters",
                example: "'https://example.com/foo%20bar' | url decode",
                result: Some(Value::test_string("https://example.com/foo bar")),
            },
            Example {
                description: "Decode multiple urls with escape characters in list",
                example: "['https://example.com/foo%20bar' 'https://example.com/a%3Eb' '%E4%B8%AD%E6%96%87%E5%AD%97/eng/12%2034'] | url decode",
                result: Some(Value::list(
                    vec![
                        Value::test_string("https://example.com/foo bar"),
                        Value::test_string("https://example.com/a>b"),
                        Value::test_string("中文字/eng/12 34"),
                    ],
                    Span::test_data(),
                )),
            },
        ]
    }
}

fn action(input: &Value, _arg: &CellPathOnlyArgs, head: Span) -> Value {
    let input_span = input.span();
    match input {
        Value::String { val, .. } => {
            let val = percent_decode_str(val).decode_utf8();
            match val {
                Ok(val) => Value::string(val, head),
                Err(e) => Value::error(
                    ShellError::GenericError {
                        error: "Failed to decode string".into(),
                        msg: e.to_string(),
                        span: Some(input_span),
                        help: None,
                        inner: vec![],
                    },
                    head,
                ),
            }
        }
        Value::Error { .. } => input.clone(),
        _ => Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: "string".into(),
                wrong_type: input.get_type().to_string(),
                dst_span: head,
                src_span: input.span(),
            },
            head,
        ),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(UrlDecode {})
    }
}
