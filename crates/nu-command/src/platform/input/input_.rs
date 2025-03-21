use crate::platform::input::legacy_input::LegacyInput;
use crate::platform::input::reedline_prompt::ReedlinePrompt;
use nu_engine::command_prelude::*;
use nu_protocol::shell_error::io::IoError;
use reedline::{FileBackedHistory, Reedline, Signal, HISTORY_SIZE};

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
            .named(
                "history",
                SyntaxShape::OneOf(vec![
                    SyntaxShape::Filepath,
                    SyntaxShape::List(Box::new(SyntaxShape::String)),
                ]),
                "path to a file to read/write history or a list of history entries",
                None,
            )
            .named(
                "max-history",
                SyntaxShape::Int,
                "The maximum number of entries to keep in the history",
                None,
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
        // Those options are not supported by reedline, default to the legacy
        // implementation
        let use_legacy = [
            call.get_flag::<String>(engine_state, stack, "bytes-until-any")?
                .is_some(),
            call.has_flag(engine_state, stack, "suppress-output")?,
            call.get_flag::<Spanned<i64>>(engine_state, stack, "numchar")?
                .is_some(),
        ]
        .iter()
        .any(|x| *x);

        if use_legacy {
            return self.legacy_input(engine_state, stack, call, _input);
        }

        let prompt_str: Option<String> = call.opt(engine_state, stack, 0)?;
        let default_val: Option<String> = call.get_flag(engine_state, stack, "default")?;
        let history_val: Option<Value> = call.get_flag(engine_state, stack, "history")?;

        let from_io_error = IoError::factory(call.head, None);

        let default_str = match (&prompt_str, &default_val) {
            (Some(_prompt), Some(val)) => format!("(default: {val}) "),
            _ => "".to_string(),
        };

        // Here we will render the default prompt to the right
        let history = match history_val {
            None => None,
            Some(Value::String { val, .. }) => Some(Box::new(
                FileBackedHistory::with_file(HISTORY_SIZE, val.into())
                    .expect("Error creating history file"),
            )),
            Some(h) => {
                return Err(ShellError::UnsupportedInput {
                    msg: "Unsupported history type".to_string(),
                    input: "value originated from here".to_string(),
                    msg_span: call.head,
                    input_span: h.span(),
                })
            } // Value::List { vals, .. } => Some(Box::new((usize::MAX, val.into()))),
        };

        let prompt = ReedlinePrompt {
            indicator: default_str,
            left_prompt: prompt_str.unwrap_or("".to_string()),
            right_prompt: "".to_string(),
        };
        let mut line_editor = Reedline::create();
        line_editor = line_editor.with_ansi_colors(false);
        line_editor = match history {
            Some(h) => line_editor.with_history(h),
            None => line_editor,
        };

        let mut buf = String::new();

        loop {
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
