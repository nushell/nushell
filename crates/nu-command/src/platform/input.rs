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

    fn signature(&self) -> Signature {
        Signature::build("input")
            .optional("prompt", SyntaxShape::String, "prompt to show the user")
            .named(
                "bytes_until",
                SyntaxShape::String,
                "read bytes (not text) until a stop byte",
                Some('u'),
            )
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
        let bytes_until: Option<String> = call.get_flag(engine_state, stack, "bytes_until")?;

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

            // Just read a normal line of text
            let mut buf = String::new();
            let input = std::io::stdin().read_line(&mut buf);

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
            example: "let user-input = (input)",
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
