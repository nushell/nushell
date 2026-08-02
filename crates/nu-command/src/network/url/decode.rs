use std::borrow::Cow;

use nu_cmd_base::input_handler::{CmdArgument, operate};
use nu_engine::command_prelude::*;

use percent_encoding::percent_decode_str;

struct Arguments {
    cell_paths: Option<Vec<CellPath>>,
    binary: bool,
}

impl CmdArgument for Arguments {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        self.cell_paths.take()
    }
}

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
                (Type::String, Type::Binary),
                (
                    Type::List(Box::new(Type::String)),
                    Type::List(Box::new(Type::String)),
                ),
                (
                    Type::List(Box::new(Type::String)),
                    Type::List(Box::new(Type::Binary)),
                ),
                (Type::table(), Type::table()),
                (Type::record(), Type::record()),
            ])
            .allow_variants_without_examples(true)
            .switch(
                "binary",
                "Return a binary value, to allow decoding non UTF-8 text.",
                Some('b'),
            )
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
        let cell_paths = Some(cell_paths).filter(|v| !v.is_empty());
        let binary = call.has_flag(engine_state, stack, "binary")?;
        let args = Arguments { cell_paths, binary };
        operate(action, args, input, call.head, engine_state.signals())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Decode a URL with escape characters.",
                example: "'https://example.com/foo%20bar' | url decode",
                result: Some(Value::test_string("https://example.com/foo bar")),
            },
            Example {
                description: "Decode multiple URLs with escape characters in list.",
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
            Example {
                description: "Decode a percent-encoded iso-8859-1 string.",
                example: "'%A3%20rates' | url decode --binary | decode iso-8859-1",
                result: Some(Value::test_string("£ rates")),
            },
        ]
    }
}

fn action(input: &Value, args: &Arguments, head: Span) -> Value {
    let input_span = input.span();
    match input {
        Value::String { val, .. } => {
            let percent_decode_str = percent_decode_str(val);
            match args.binary {
                true => {
                    let data: Cow<'_, [u8]> = percent_decode_str.into();
                    Value::binary(data, head)
                }
                false => {
                    let val = percent_decode_str.decode_utf8();
                    match val {
                        Ok(val) => Value::string(val, head),
                        Err(_) => Value::error(
                            ShellError::NonUtf8Custom {
                                msg: "\
                                    Input is not UTF-8 encoded.\n\
                                    Try using the `--binary` flag together with `decode`."
                                    .into(),
                                span: input_span,
                            },
                            head,
                        ),
                    }
                }
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
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(UrlDecode)
    }
}
