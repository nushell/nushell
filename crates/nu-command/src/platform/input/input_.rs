use crossterm::{
    cursor, execute,
    style::Print,
    terminal::{self, ClearType},
};
use itertools::Itertools;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Spanned, SyntaxShape,
    Type, Value,
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
            .input_output_types(vec![
                (Type::Nothing, Type::String),
                (Type::Nothing, Type::Binary),
            ])
            .allow_variants_without_examples(true)
            .optional("prompt", SyntaxShape::String, "prompt to show the user")
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
        let bytes_until: Option<String> = call.get_flag(engine_state, stack, "bytes-until-any")?;
        let suppress_output = call.has_flag("suppress-output");
        let numchar: Option<Spanned<i64>> = call.get_flag(engine_state, stack, "numchar")?;
        let numchar: Spanned<i64> = numchar.unwrap_or(Spanned {
            item: i64::MAX,
            span: call.head,
        });

        if numchar.item < 1 {
            return Err(ShellError::UnsupportedInput(
                "Number of characters to read has to be positive".to_string(),
                "value originated from here".to_string(),
                call.head,
                numchar.span,
            ));
        }

        if let Some(prompt) = &prompt {
            print!("{prompt}");
            let _ = std::io::stdout().flush();
        }

        let mut b = [0u8; 1];
        let mut buf = vec![];

        crossterm::terminal::enable_raw_mode()?;
        let mut stdin = std::io::stdin();

        loop {
            if i64::try_from(buf.len()).unwrap_or(0) >= numchar.item {
                break;
            }

            if let Err(err) = stdin.read_exact(&mut b) {
                crossterm::terminal::disable_raw_mode()?;
                return Err(ShellError::IOError(err.to_string()));
            }

            buf.push(b[0]);

            if let Some(bytes_until) = bytes_until.as_ref() {
                if bytes_until.bytes().contains(&b[0]) {
                    break;
                }
            }

            // 03 symbolizes SIGINT/Ctrl+C
            if buf.contains(&3) {
                let _ = crossterm::terminal::disable_raw_mode();
                return Err(ShellError::IOError("SIGINT".to_string()));
            }

            if !suppress_output {
                // clear the current line and print the current buffer
                execute!(
                    std::io::stdout(),
                    terminal::Clear(ClearType::CurrentLine),
                    cursor::MoveToColumn(0),
                )?;
                if let Some(prompt) = &prompt {
                    execute!(std::io::stdout(), Print(prompt.to_string()))?;
                }
                if let Ok(s) = std::str::from_utf8(&buf) {
                    execute!(std::io::stdout(), Print(s))?;
                }
            }
        }
        crossterm::terminal::disable_raw_mode()?;
        if !suppress_output {
            std::io::stdout().write_all(b"\n")?;
        }

        Ok(Value::Binary {
            val: buf,
            internal_span: call.head,
        }
        .into_pipeline_data())
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
