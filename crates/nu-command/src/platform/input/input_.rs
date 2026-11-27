use crate::platform::input::legacy_input::LegacyInput;
use crate::platform::input::reedline_prompt::{ModeIndicators, ReedlinePrompt};
use nu_engine::{ClosureEval, command_prelude::*};
use nu_protocol::engine::Closure;
use nu_protocol::shell_error::{self, io::IoError};
use reedline::{
    ColumnarMenu, CompletionResult, EditCommand, Emacs, FileBackedHistory, HISTORY_SIZE, History,
    HistoryItem, KeyCode, KeyModifiers, MenuBuilder, Reedline, ReedlineEvent, ReedlineMenu, Signal,
    Suggestion, default_emacs_keybindings,
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
            .named(
                "completer",
                SyntaxShape::Closure(Some(vec![SyntaxShape::String, SyntaxShape::Int])),
                "Closure called with the current buffer and cursor position when the completion menu is triggered (Tab). Returns completions as a list of strings, or of records with a `value` column and optional `description`, `start`, and `end` columns. Implies `--reedline`.",
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
        let completer_closure: Option<Closure> = call.get_flag(engine_state, stack, "completer")?;

        // Check if we should use the legacy implementation or the reedline implementation
        let use_reedline = [
            // reedline is not set - use legacy implementation
            call.has_flag(engine_state, stack, "reedline")?,
            // We have the history-file or max-history flags set to None
            call.get_flag::<String>(engine_state, stack, "history-file")?
                .is_some(),
            call.get_flag::<i64>(engine_state, stack, "max-history")?
                .is_some(),
            completer_closure.is_some(),
        ]
        .iter()
        .any(|x| *x);

        if !use_reedline {
            // `legacy_input` guards the terminal itself via `RawModeGuard`.
            return self.legacy_input(engine_state, stack, call, input);
        }

        // Reedline grabs the terminal itself, so check the precondition here.
        stack.require_stdin(call.head)?;

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

        let config = stack.get_config(engine_state);
        let indicators = ModeIndicators::from_config(&config, engine_state, stack);

        let prompt = ReedlinePrompt {
            indicator: default_str,
            left_prompt: prompt_str.unwrap_or("".to_string()),
            right_prompt: "".to_string(),
            indicators,
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

        if let Some(closure) = completer_closure {
            let closure_eval = ClosureEval::try_new(engine_state, stack, closure, call.head)?;
            let signature = &closure_eval.block().signature;
            let num_params =
                signature.required_positional.len() + signature.optional_positional.len();
            let completer = ClosureCompleter {
                closure_eval,
                num_params,
                span: call.head,
            };
            // A completer only does something through a menu and a key bound to
            // open it, which the plain `Reedline::create()` editor lacks; wire
            // up the same defaults the shell uses: a columnar menu on Tab.
            let completion_menu = ColumnarMenu::default().with_name("completion_menu");
            let mut keybindings = default_emacs_keybindings();
            keybindings.add_binding(
                KeyModifiers::NONE,
                KeyCode::Tab,
                ReedlineEvent::UntilFound(vec![
                    ReedlineEvent::Menu("completion_menu".to_string()),
                    ReedlineEvent::MenuNext,
                ]),
            );
            keybindings.add_binding(
                KeyModifiers::SHIFT,
                KeyCode::BackTab,
                ReedlineEvent::MenuPrevious,
            );
            line_editor = line_editor
                .with_completer(Box::new(completer))
                .with_menu(ReedlineMenu::EngineCompleter(Box::new(completion_menu)))
                .with_edit_mode(Box::new(Emacs::new(keybindings)))
                .with_quick_completions(true)
                .with_partial_completions(true);
        }

        let mut buf = String::new();

        match line_editor.read_line(&prompt) {
            Ok(Signal::Success(buffer) | Signal::HostCommand(buffer)) => {
                buf.push_str(&buffer);
            }
            Ok(Signal::CtrlC(_)) => {
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
            // TODO: handle other signals like Signal::ExternalBreak
            Ok(_) => {}
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
            Example {
                description: "Get input from the user with a custom completion menu on Tab.",
                example: "let user_input = (input --completer {|buffer, pos| ls | get name })",
                result: None,
            },
        ]
    }
}

/// Adapts a user-supplied closure to reedline's [`Completer`] trait.
///
/// The completer is called re-entrantly from inside `read_line`, so it owns
/// its evaluation state ([`ClosureEval`] keeps an owned `EngineState` clone);
/// environment changes made by the closure do not propagate back out.
struct ClosureCompleter {
    closure_eval: ClosureEval,
    /// Number of positional parameters the closure declares: the buffer is
    /// always passed, the cursor position only if a second parameter exists.
    num_params: usize,
    span: Span,
}

impl ClosureCompleter {
    fn try_complete(&mut self, line: &str, pos: usize) -> Result<Vec<Suggestion>, ShellError> {
        let pos = pos.min(line.len());
        self.closure_eval.add_arg(Value::string(line, self.span))?;
        if self.num_params >= 2 {
            self.closure_eval
                .add_arg(Value::int(pos as i64, self.span))?;
        }
        let vals = self
            .closure_eval
            .run_with_input(PipelineData::empty())?
            .into_value(self.span)?
            .into_list()?;

        // Default replacement span: the trailing word, scanning back from the
        // cursor to the nearest whitespace. Records may override it with
        // `start`/`end` columns.
        let word_start = line[..pos]
            .rfind(char::is_whitespace)
            .map(|i| i + 1)
            .unwrap_or(0);

        Ok(vals
            .into_iter()
            .filter_map(|val| match val {
                Value::String { val, .. } => Some(Suggestion {
                    value: val,
                    span: reedline::Span::new(word_start, pos),
                    ..Suggestion::default()
                }),
                Value::Record { val, .. } => {
                    let get_pos = |col: &str| {
                        val.get(col)
                            .and_then(|v| v.as_int().ok())
                            .map(|i| (i.max(0) as usize).min(line.len()))
                    };
                    let start = get_pos("start").unwrap_or(word_start);
                    let end = get_pos("end").unwrap_or(pos).max(start);
                    Some(Suggestion {
                        value: val.get("value")?.coerce_string().ok()?,
                        description: val.get("description").and_then(|v| v.coerce_string().ok()),
                        span: reedline::Span::new(start, end),
                        ..Suggestion::default()
                    })
                }
                _ => None,
            })
            .collect())
    }
}

impl reedline::Completer for ClosureCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> CompletionResult {
        // A completer must not fail mid-edit; on error, offer no completions.
        CompletionResult::fresh(self.try_complete(line, pos).unwrap_or_default())
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
    fn examples_work_as_expected() -> nu_test_support::Result {
        nu_test_support::test().examples(Input)
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
