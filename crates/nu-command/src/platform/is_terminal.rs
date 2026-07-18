//! OS-level TTY check for process stdio (`isatty`).
//!
//! Complements [`super::is_redirected`]: this command only inspects the process
//! file descriptors. It does **not** consult Nushell pipeline destinations
//! (`OutDest`), so it works inside `if (is-terminal)` and for scripts such as
//! `./script.nu | cat`.

use nu_engine::command_prelude::*;
use std::io::IsTerminal as _;

#[derive(Clone)]
pub struct IsTerminal;

impl Command for IsTerminal {
    fn name(&self) -> &str {
        "is-terminal"
    }

    fn signature(&self) -> Signature {
        Signature::build("is-terminal")
            .input_output_type(Type::Nothing, Type::Bool)
            .switch("stdin", "Check if stdin is a terminal.", Some('i'))
            .switch("stdout", "Check if stdout is a terminal.", Some('o'))
            .switch("stderr", "Check if stderr is a terminal.", Some('e'))
            .category(Category::Platform)
    }

    fn description(&self) -> &str {
        "Check if the process stdin, stdout, or stderr is attached to a terminal device."
    }

    fn extra_description(&self) -> &str {
        "This is an operating-system level check (like bash `test -t`), not a Nushell
pipeline check. It answers whether the process file descriptor is a TTY, which is
what scripts need for `./script.nu | cat` vs running on a terminal.

For detecting whether a custom command's return value is piped or collected inside
Nushell (pretty output vs structured data), use `is-redirected` instead."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Check if stdout is a terminal (default when no flag is specified).",
                example: "is-terminal",
                result: None,
            },
            Example {
                description: r#"Return "terminal attached" if standard input is attached to a terminal, and "no terminal" if not."#,
                example: r#"if (is-terminal --stdin) { "terminal attached" } else { "no terminal" }"#,
                result: Some(Value::test_string("terminal attached")),
            },
            Example {
                description: "Choose formatting based on whether process stdout is a TTY (works inside `if`).",
                example: r#"if (is-terminal --stdout) { "human" } else { "piped" }"#,
                result: None,
            },
        ]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec![
            "input", "output", "stdin", "stdout", "stderr", "tty", "isatty",
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let stdin = call.has_flag(engine_state, stack, "stdin")?;
        let stdout = call.has_flag(engine_state, stack, "stdout")?;
        let stderr = call.has_flag(engine_state, stack, "stderr")?;

        // Default (no flags) is stdout, matching bash `test -t 1`.
        let is_terminal = match (stdin, stdout, stderr) {
            (true, false, false) => std::io::stdin().is_terminal(),
            (false, false, true) => std::io::stderr().is_terminal(),
            (false, true, false) | (false, false, false) => std::io::stdout().is_terminal(),
            _ => {
                return Err(ShellError::IncompatibleParametersSingle {
                    msg: "Only one stream may be checked".into(),
                    span: call.arguments_span(),
                });
            }
        };

        Ok(PipelineData::value(
            Value::bool(is_terminal, call.head),
            None,
        ))
    }
}
