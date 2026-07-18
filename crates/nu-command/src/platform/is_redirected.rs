//! Nushell pipeline-destination check for custom commands.
//!
//! Complements [`super::is_terminal`]: `is-terminal` is OS `isatty`, while this command
//! answers whether a custom command's *return value* is piped, collected, filed, or
//! discarded rather than printed.
//!
//! Implementation relies on [`Stack::is_stdout_redirected`], which reads the
//! invocation-stdout frame pushed in `eval_call` via [`Stack::with_invocation_stdout`].

use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct IsRedirected;

impl Command for IsRedirected {
    fn name(&self) -> &str {
        "is-redirected"
    }

    fn signature(&self) -> Signature {
        Signature::build("is-redirected")
            .input_output_type(Type::Nothing, Type::Bool)
            .category(Category::Platform)
    }

    fn description(&self) -> &str {
        "Check if the current custom command's return value is redirected away from display."
    }

    fn extra_description(&self) -> &str {
        "This is a Nushell pipeline-destination check, not an OS TTY check.

Inside a custom command, `is-redirected` reports whether that command's return
value will be piped to another command, collected into a value (`let`,
subexpression), written to a file, or discarded — as opposed to being printed
via the normal display path.

Unlike looking at process stdout, this is stable for the whole command body, so
it works inside `if (...)` and `let x = (...)`.

For whether the process stdout is a terminal (scripts: `./script | cat`), use
`is-terminal --stdout` instead.

Typical pattern for pretty-vs-data custom commands:

    def mycmd [] {
      if (is-terminal --stdout) and not (is-redirected) {
        # human-friendly formatting
      } else {
        # structured data for pipelines
      }
    }
"
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Inside a custom command, report whether the call's return value is redirected (works in `if`).",
                example: r#"def pipetest [] { if (is-redirected) { "piped" } else { "display" } }; pipetest"#,
                // Display vs redirect depends on how the call is invoked; result omitted.
                result: None,
            },
            Example {
                description: "Return true when the custom command is piped to another command.",
                example: "def pipetest [] { is-redirected }; pipetest | $in",
                result: Some(Value::test_bool(true)),
            },
            Example {
                description: "Return true when the custom command result is collected into a variable.",
                example: "def pipetest [] { is-redirected }; let x = (pipetest); $x",
                result: Some(Value::test_bool(true)),
            },
        ]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec![
            "pipe",
            "pipeline",
            "redirect",
            "redirection",
            "display",
            "stdout",
            "tty",
        ]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(PipelineData::value(
            Value::bool(stack.is_stdout_redirected(), call.head),
            None,
        ))
    }
}
