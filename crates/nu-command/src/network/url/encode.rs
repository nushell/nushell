use crate::input_handler::{operate, CellPathOnlyArgs};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::ast::CellPath;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::Category;
use nu_protocol::{Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value};
use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "url encode"
    }

    fn signature(&self) -> Signature {
        Signature::build("url encode")
            .input_output_types(vec![(Type::String, Type::String)])
            .vectorizes_over_list(true)
            .switch(
            "all", 
            "encode all non-alphanumeric chars including `/`, `.`, `:`",
            Some('a'))
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, check strings at the given cell paths, and replace with result",
            )
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Converts a string to a percent encoded web safe string"
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
        if call.has_flag("all") {
            operate(
                action_all,
                args,
                input,
                call.head,
                engine_state.ctrlc.clone(),
            )
        } else {
            operate(action, args, input, call.head, engine_state.ctrlc.clone())
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Encode a url with escape characters",
                example: "'https://example.com/foo bar' | url encode",
                result: Some(Value::test_string("https://example.com/foo%20bar")),
            },
            Example {
                description: "Encode multiple urls with escape characters in list",
                example: "['https://example.com/foo bar' 'https://example.com/a>b' '中文字/eng/12 34'] | url encode",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_string("https://example.com/foo%20bar"),
                        Value::test_string("https://example.com/a%3Eb"),
                        Value::test_string("%E4%B8%AD%E6%96%87%E5%AD%97/eng/12%2034"),
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Encode all non anphanumeric chars with all flag",
                example: "'https://example.com/foo bar' | url encode --all",
                result: Some(Value::test_string("https%3A%2F%2Fexample%2Ecom%2Ffoo%20bar")),
            },
        ]
    }
}

fn action_all(input: &Value, _arg: &CellPathOnlyArgs, head: Span) -> Value {
    match input {
        Value::String { val, .. } => {
            const FRAGMENT: &AsciiSet = NON_ALPHANUMERIC;
            Value::String {
                val: utf8_percent_encode(val, FRAGMENT).to_string(),
                span: head,
            }
        }
        Value::Error { .. } => input.clone(),
        _ => Value::Error {
            error: ShellError::OnlySupportsThisInputType(
                "string".into(),
                input.get_type().to_string(),
                head,
                input.expect_span(),
            ),
        },
    }
}

fn action(input: &Value, _arg: &CellPathOnlyArgs, head: Span) -> Value {
    match input {
        Value::String { val, .. } => {
            const FRAGMENT: &AsciiSet = &NON_ALPHANUMERIC.remove(b'/').remove(b':').remove(b'.');
            Value::String {
                val: utf8_percent_encode(val, FRAGMENT).to_string(),
                span: head,
            }
        }
        Value::Error { .. } => input.clone(),
        _ => Value::Error {
            error: ShellError::OnlySupportsThisInputType(
                "string".into(),
                input.get_type().to_string(),
                head,
                input.expect_span(),
            ),
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
