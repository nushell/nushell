use nu_cmd_base::input_handler::{CmdArgument, operate};
use nu_engine::command_prelude::*;

use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, percent_encode, utf8_percent_encode};

struct Arguments {
    cell_paths: Option<Vec<CellPath>>,
    ascii_set: &'static AsciiSet,
}

static ASCII_SET_ALL: &AsciiSet = NON_ALPHANUMERIC;
static ASCII_SET_NOT_ALL: &AsciiSet = &NON_ALPHANUMERIC.remove(b'/').remove(b':').remove(b'.');

impl CmdArgument for Arguments {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        self.cell_paths.take()
    }
}

#[derive(Clone)]
pub struct UrlEncode;

impl Command for UrlEncode {
    fn name(&self) -> &str {
        "url encode"
    }

    fn signature(&self) -> Signature {
        Signature::build("url encode")
            .input_output_types(vec![
                (Type::String, Type::String),
                (Type::Binary, Type::String),
                (Type::List(Box::new(Type::one_of([Type::String, Type::Binary]))), Type::List(Box::new(Type::String))),
                (Type::table(), Type::table()),
                (Type::record(), Type::record()),
            ])
            .allow_variants_without_examples(true)
            .switch(
            "all",
            "Encode all non-alphanumeric chars including `/`, `.`, `:`.",
            Some('a'))
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, check strings at the given cell paths, and replace with result.",
            )
            .category(Category::Strings)
    }

    fn description(&self) -> &str {
        "Converts a string to a percent encoded web safe string."
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
        let ascii_set = match call.has_flag(engine_state, stack, "all")? {
            true => ASCII_SET_ALL,
            false => ASCII_SET_NOT_ALL,
        };
        let args = Arguments {
            cell_paths,
            ascii_set,
        };
        operate(action, args, input, call.head, engine_state.signals())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Encode a URL with escape characters.",
                example: "'https://example.com/foo bar' | url encode",
                result: Some(Value::test_string("https://example.com/foo%20bar")),
            },
            Example {
                description: "Encode multiple URLs with escape characters in list.",
                example: "['https://example.com/foo bar' 'https://example.com/a>b' '中文字/eng/12 34'] | url encode",
                result: Some(Value::list(
                    vec![
                        Value::test_string("https://example.com/foo%20bar"),
                        Value::test_string("https://example.com/a%3Eb"),
                        Value::test_string("%E4%B8%AD%E6%96%87%E5%AD%97/eng/12%2034"),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Encode all non alphanumeric chars with all flag.",
                example: "'https://example.com/foo bar' | url encode --all",
                result: Some(Value::test_string(
                    "https%3A%2F%2Fexample%2Ecom%2Ffoo%20bar",
                )),
            },
            Example {
                description: "Encode a iso-8859-1 encoded string.",
                example: "'£ rates' | encode iso-8859-1 | url encode",
                result: Some(Value::test_string("%A3%20rates")),
            },
        ]
    }
}

fn action(input: &Value, args: &Arguments, head: Span) -> Value {
    match input {
        Value::String { val, .. } => {
            let utf8_percent_encode = utf8_percent_encode(val, args.ascii_set);
            Value::string(utf8_percent_encode.to_string(), head)
        }
        Value::Binary { val, .. } => {
            let utf8_percent_encode = percent_encode(val, args.ascii_set);
            Value::string(utf8_percent_encode.to_string(), head)
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
        nu_test_support::test().examples(UrlEncode)
    }
}
