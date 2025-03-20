use crate::platform::input::legacy_input::LegacyInput;
use crate::platform::input::reedline_prompt::ReedlinePrompt;
use nu_engine::command_prelude::*;
use nu_protocol::shell_error::io::IoError;
use reedline::{Reedline, Signal};

use std::io::Write;

#[derive(Clone)]
pub struct Input;

impl LegacyInput for Input {}

impl Command for Input {
    fn name(&self) -> &str {
        "input"
    }

    fn description(&self) -> &str {
        "Get input from the user."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["prompt", "interactive"]
    }

    fn signature(&self) -> Signature {
        Signature::build("input")
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .allow_variants_without_examples(true)
            .optional("prompt", SyntaxShape::String, "Prompt to show the user.")
            .named(
                "bytes-until-any",
                SyntaxShape::String,
                "read bytes (not text) until any of the given stop bytes is seen",
                Some('u'),
            )
            .named(
                "numchar",
                SyntaxShape::Int,
                "number of characters to read; suppresses output",
                Some('n'),
            )
            .named(
                "default",
                SyntaxShape::String,
                "default value if no input is provided",
                Some('d'),
            )
            .switch("suppress-output", "don't print keystroke values", Some('s'))
            .category(Category::Platform)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let prompt_str: Option<String> = call.opt(engine_state, stack, 0)?;
        let bytes_until: Option<String> = call.get_flag(engine_state, stack, "bytes-until-any")?;
        let suppress_output = call.has_flag(engine_state, stack, "suppress-output")?;
        let numchar_flag: Option<Spanned<i64>> = call.get_flag(engine_state, stack, "numchar")?;
        let numchar: Spanned<i64> = numchar_flag.unwrap_or(Spanned {
            item: i64::MAX,
            span: call.head,
        });

        let from_io_error = IoError::factory(call.head, None);

        if numchar.item < 1 {
            return Err(ShellError::UnsupportedInput {
                msg: "Number of characters to read has to be positive".to_string(),
                input: "value originated from here".to_string(),
                msg_span: call.head,
                input_span: numchar.span,
            });
        }

        // Those 2 options are not supported by reedline, default to the legacy
        // implementation
        if suppress_output || bytes_until.is_some() || numchar_flag.is_some() {
            return self.legacy_input(engine_state, stack, call, _input);
        }

        let default_val: Option<String> = call.get_flag(engine_state, stack, "default")?;

        if let Some(prompt) = &prompt_str {
            match &default_val {
                None => print!("{prompt}"),
                Some(val) => print!("{prompt} (default: {val})"),
            }
            let _ = std::io::stdout().flush();
        }

        let mut buf = String::new();
        let prompt = ReedlinePrompt {
            left_prompt: prompt_str.unwrap_or("".to_string()),
            indicator: "".to_string(), // TODO: Add support for custom prompt indicators
                                       // for now, and backwards compat, we just use the  empty
                                       // string
        };
        let mut line_editor = Reedline::create();
        // TODO handle options

        loop {
            if i64::try_from(buf.len()).unwrap_or(0) >= numchar.item {
                break;
            }
            match line_editor.read_line(&prompt) {
                Ok(Signal::Success(buffer)) => {
                    buf.push_str(&buffer);
                    break;
                }
                Ok(Signal::CtrlC) => {
                    return Err(
                        IoError::new(std::io::ErrorKind::Interrupted, call.head, None).into(),
                    );
                }
                Ok(_) => continue,
                Err(event_error) => {
                    crossterm::terminal::disable_raw_mode().map_err(&from_io_error)?;
                    return Err(from_io_error(event_error).into());
                }
            }
        }
        match default_val {
            Some(val) if buf.is_empty() => Ok(Value::string(val, call.head).into_pipeline_data()),
            _ => Ok(Value::string(buf, call.head).into_pipeline_data()),
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get input from the user, and assign to a variable",
                example: "let user_input = (input)",
                result: None,
            },
            Example {
                description: "Get two characters from the user, and assign to a variable",
                example: "let user_input = (input --numchar 2)",
                result: None,
            },
            Example {
                description: "Get input from the user with default value, and assign to a variable",
                example: "let user_input = (input --default 10)",
                result: None,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::Input;

    #[test]
    fn examples_work_as_expected() {
        use crate::test_examples;
        test_examples(Input {})
    }
}
