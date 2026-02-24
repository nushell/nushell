use crate::platform::input::legacy_input::LegacyInput;
use crate::platform::input::reedline_prompt::ReedlinePrompt;
use nu_engine::command_prelude::*;
use nu_protocol::shell_error::{self, io::IoError};
use reedline::{
    EditCommand, FileBackedHistory, HISTORY_SIZE, History, HistoryItem, Reedline, Signal,
};

#[derive(Clone)]
pub struct Input;

impl LegacyInput for Input {}

impl Command for Input {
    fn name(&self) -> &str {
        "input"
    }

    fn description(&self) -> &str {
        "Get input from the user via the terminal."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["prompt", "interactive"]
    }

    fn signature(&self) -> Signature {
        Signature::build("input")
            .input_output_types(vec![
                (Type::Nothing, Type::Any),
                (Type::List(Box::new(Type::String)), Type::Any)])
            .allow_variants_without_examples(true)
            .optional("prompt", SyntaxShape::String, "Prompt to show the user.")
            .named(
                "bytes-until-any",
                SyntaxShape::String,
                "Read bytes (not text) until any of the given stop bytes is seen.",
                Some('u'),
            )
            .named(
                "numchar",
                SyntaxShape::Int,
                "Number of characters to read; suppresses output.",
                Some('n'),
            )
            .named(
                "default",
                SyntaxShape::String,
                "Default value if no input is provided.",
                Some('d'),
            )
            .switch(
                "reedline",
                "Use the reedline library, defaults to false.",
                None
            )
            .named(
                "history-file",
                SyntaxShape::Filepath,
                "Path to a file to read and write command history. This is a text file and will be created if it doesn't exist. Will be used as the selection list. Implies `--reedline`.",
                None,
            )
            .named(
                "max-history",
                SyntaxShape::Int,
                "The maximum number of entries to keep in the history, defaults to $env.config.history.max_size. Implies `--reedline`.",
                None,
            )
            .switch("suppress-output", "Don't print keystroke values.", Some('s'))
            .category(Category::Platform)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // Check if we should use the legacy implementation or the reedline implementation
        let use_reedline = [
            // reedline is not set - use legacy implementation
            call.has_flag(engine_state, stack, "reedline")?,
            // We have the history-file or max-history flags set to None
            call.get_flag::<String>(engine_state, stack, "history-file")?
                .is_some(),
            call.get_flag::<i64>(engine_state, stack, "max-history")?
                .is_some(),
        ]
        .iter()
        .any(|x| *x);

        if !use_reedline {
            return self.legacy_input(engine_state, stack, call, input);
        }

        let prompt_str: Option<String> = call.opt(engine_state, stack, 0)?;
        let default_val: Option<String> = call.get_flag(engine_state, stack, "default")?;
        let history_file_val: Option<String> =
            call.get_flag(engine_state, stack, "history-file")?;
        let max_history: usize = call
            .get_flag::<i64>(engine_state, stack, "max-history")?
            .map(|l| if l < 0 { 0 } else { l as usize })
            .unwrap_or(HISTORY_SIZE);
        let max_history_span = call.get_flag_span(stack, "max-history");
        let history_file_span = call.get_flag_span(stack, "history-file");

        let default_str = match (&prompt_str, &default_val) {
            (Some(_prompt), Some(val)) => format!("(default: {val}) "),
            _ => "".to_string(),
        };

        let history_entries = match input {
            PipelineData::Value(Value::List { vals, .. }, ..) => Some(vals),
            _ => None,
        };

        // If we either have history entries or history file, we create an history
        let history = match (history_entries.is_some(), history_file_val.is_some()) {
            (false, false) => None, // Neither are set, no need for history support
            _ => {
                let file_history = match history_file_val {
                    Some(file) => FileBackedHistory::with_file(max_history, file.into()),
                    None => FileBackedHistory::new(max_history),
                };
                let mut history = match file_history {
                    Ok(h) => h,
                    Err(e) => match e.0 {
                        reedline::ReedlineErrorVariants::IOError(err) => {
                            return Err(ShellError::IncorrectValue {
                                msg: err.to_string(),
                                val_span: history_file_span.expect("history-file should be set"),
                                call_span: call.head,
                            });
                        }
                        reedline::ReedlineErrorVariants::OtherHistoryError(msg) => {
                            return Err(ShellError::IncorrectValue {
                                msg: msg.to_string(),
                                val_span: max_history_span.expect("max-history should be set"),
                                call_span: call.head,
                            });
                        }
                        _ => {
                            return Err(ShellError::IncorrectValue {
                                msg: "unable to create history".to_string(),
                                val_span: call.head,
                                call_span: call.head,
                            });
                        }
                    },
                };

                if let Some(vals) = history_entries {
                    vals.iter().for_each(|val| {
                        if let Value::String { val, .. } = val {
                            let _ = history.save(HistoryItem::from_command_line(val.clone()));
                        }
                    });
                }
                Some(history)
            }
        };

        let prompt = ReedlinePrompt {
            indicator: default_str,
            left_prompt: prompt_str.unwrap_or("".to_string()),
            right_prompt: "".to_string(),
        };

        let mut line_editor = Reedline::create();
        line_editor = line_editor.with_ansi_colors(false);
        line_editor = match history {
            Some(h) => line_editor.with_history(Box::new(h)),
            None => line_editor,
        };

        // In reedline mode, treat `--default` as the initial editable buffer contents.
        // This keeps options minimal while supporting the "prefilled but editable" UX.
        if let Some(val) = default_val.as_ref() {
            prefill_reedline_buffer(&mut line_editor, val);
        }

        let mut buf = String::new();

        match line_editor.read_line(&prompt) {
            Ok(Signal::Success(buffer)) => {
                buf.push_str(&buffer);
            }
            Ok(Signal::CtrlC) => {
                return Err(IoError::new(
                    shell_error::io::ErrorKind::from_std(std::io::ErrorKind::Interrupted),
                    call.head,
                    None,
                )
                .into());
            }
            Ok(Signal::CtrlD) => {
                // Do nothing on ctrl-d
                return Ok(Value::nothing(call.head).into_pipeline_data());
            }
            Err(event_error) => {
                let from_io_error = IoError::factory(call.head, None);
                return Err(from_io_error(event_error).into());
            }
        }
        match default_val {
            Some(val) if buf.is_empty() => Ok(Value::string(val, call.head).into_pipeline_data()),
            _ => Ok(Value::string(buf, call.head).into_pipeline_data()),
        }
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Get input from the user, and assign to a variable.",
                example: "let user_input = (input)",
                result: None,
            },
            Example {
                description: "Get two characters from the user, and assign to a variable.",
                example: "let user_input = (input --numchar 2)",
                result: None,
            },
            Example {
                description: "Get input from the user with default value, and assign to a variable.",
                example: "let user_input = (input --default 10)",
                result: None,
            },
            Example {
                description: "Get multiple lines of input from the user (newlines can be entered using `Alt` + `Enter` or `Ctrl` + `Enter`), and assign to a variable.",
                example: "let multiline_input = (input --reedline)",
                result: None,
            },
            Example {
                description: "Get input from the user with history, and assign to a variable.",
                example: "let user_input = ([past,command,entries] | input --reedline)",
                result: None,
            },
            Example {
                description: "Get input from the user with history backed by a file, and assign to a variable.",
                example: "let user_input = (input --reedline --history-file ./history.txt)",
                result: None,
            },
        ]
    }
}

fn prefill_reedline_buffer(line_editor: &mut Reedline, default_val: &str) {
    if default_val.is_empty() {
        return;
    }

    // Start with a clean buffer. This also ensures idempotency if this function is ever called
    // more than once.
    line_editor.run_edit_commands(&[EditCommand::Clear]);
    line_editor.run_edit_commands(&[EditCommand::InsertString(default_val.to_string())]);
    // Keep cursor at end (insertion point is naturally advanced by InsertString).
}

#[cfg(test)]
mod tests {
    use super::Input;
    use super::prefill_reedline_buffer;
    use reedline::Reedline;

    #[test]
    fn examples_work_as_expected() {
        use crate::test_examples;
        test_examples(Input {})
    }

    #[test]
    fn reedline_default_prefills_editable_buffer() {
        let mut line_editor = Reedline::create();
        prefill_reedline_buffer(&mut line_editor, "foobar.txt");

        assert_eq!(line_editor.current_buffer_contents(), "foobar.txt");
        assert_eq!(line_editor.current_insertion_point(), "foobar.txt".len());
    }

    #[test]
    fn reedline_default_empty_does_not_prefill() {
        let mut line_editor = Reedline::create();
        prefill_reedline_buffer(&mut line_editor, "");

        assert_eq!(line_editor.current_buffer_contents(), "");
        assert_eq!(line_editor.current_insertion_point(), 0);
    }
}
