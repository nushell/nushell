use crossterm::event::{Event, KeyCode, KeyModifiers};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};
use std::io::{Read, Write};

#[derive(Clone)]
pub struct Input;

impl Command for Input {
    fn name(&self) -> &str {
        "input"
    }

    fn usage(&self) -> &str {
        "Get input from the user."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["prompt", "interactive"]
    }

    fn signature(&self) -> Signature {
        Signature::build("input")
            .optional("prompt", SyntaxShape::String, "prompt to show the user")
            .named(
                "bytes-until",
                SyntaxShape::String,
                "read bytes (not text) until a stop byte",
                Some('u'),
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
        let prompt: Option<String> = call.opt(engine_state, stack, 0)?;
        let bytes_until: Option<String> = call.get_flag(engine_state, stack, "bytes-until")?;
        let suppress_output = call.has_flag("suppress-output");

        if let Some(bytes_until) = bytes_until {
            let _ = crossterm::terminal::enable_raw_mode();

            if let Some(prompt) = prompt {
                print!("{}", prompt);
                let _ = std::io::stdout().flush();
            }
            if let Some(c) = bytes_until.bytes().next() {
                let mut buf = [0u8; 1];
                let mut buffer = vec![];

                let mut stdin = std::io::stdin();

                loop {
                    if let Err(err) = stdin.read_exact(&mut buf) {
                        let _ = crossterm::terminal::disable_raw_mode();
                        return Err(ShellError::IOError(err.to_string()));
                    }
                    buffer.push(buf[0]);

                    if buf[0] == c {
                        let _ = crossterm::terminal::disable_raw_mode();
                        break;
                    }
                }

                Ok(Value::Binary {
                    val: buffer,
                    span: call.head,
                }
                .into_pipeline_data())
            } else {
                let _ = crossterm::terminal::disable_raw_mode();
                Err(ShellError::IOError(
                    "input can't stop on this byte".to_string(),
                ))
            }
        } else {
            if let Some(prompt) = prompt {
                print!("{}", prompt);
                let _ = std::io::stdout().flush();
            }

            let mut buf = String::new();

            if suppress_output {
                crossterm::terminal::enable_raw_mode()?;
                loop {
                    match crossterm::event::read() {
                        Ok(Event::Key(k)) => match k.code {
                            // TODO: maintain keycode parity with existing command
                            KeyCode::Char(_)
                                if k.modifiers == KeyModifiers::ALT
                                    || k.modifiers == KeyModifiers::CONTROL =>
                            {
                                continue
                            }
                            KeyCode::Char(c) => buf.push(c),
                            KeyCode::Backspace => {
                                let _ = buf.pop();
                            }
                            KeyCode::Enter => break,
                            _ => continue,
                        },
                        Ok(_) => continue,
                        Err(event_error) => {
                            crossterm::terminal::disable_raw_mode()?;
                            return Err(event_error.into());
                        }
                    }
                }
                crossterm::terminal::disable_raw_mode()?;
                return Ok(Value::String {
                    val: buf,
                    span: call.head,
                }
                .into_pipeline_data());
            }

            // Just read a normal line of text, and trim the newline at the end
            let input = std::io::stdin().read_line(&mut buf);
            if buf.ends_with('\n') {
                buf.pop();
                if buf.ends_with('\r') {
                    buf.pop();
                }
            }

            match input {
                Ok(_) => Ok(Value::String {
                    val: buf,
                    span: call.head,
                }
                .into_pipeline_data()),
                Err(err) => Err(ShellError::IOError(err.to_string())),
            }
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Get input from the user, and assign to a variable",
            example: "let user_input = (input)",
            result: None,
        }]
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
