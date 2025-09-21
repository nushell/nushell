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
            .switch("stdin", "Check if stdin is a terminal", Some('i'))
            .switch("stdout", "Check if stdout is a terminal", Some('o'))
            .switch("stderr", "Check if stderr is a terminal", Some('e'))
            .category(Category::Platform)
    }

    fn description(&self) -> &str {
        "Check if stdin, stdout, or stderr is a terminal."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: r#"Return "terminal attached" if standard input is attached to a terminal, and "no terminal" if not."#,
            example: r#"if (is-terminal --stdin) { "terminal attached" } else { "no terminal" }"#,
            result: Some(Value::test_string("terminal attached")),
        }]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["input", "output", "stdin", "stdout", "stderr", "tty"]
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

        let is_terminal = match (stdin, stdout, stderr) {
            (true, false, false) => std::io::stdin().is_terminal(),
            (false, true, false) => std::io::stdout().is_terminal(),
            (false, false, true) => std::io::stderr().is_terminal(),
            (false, false, false) => {
                return Err(ShellError::MissingParameter {
                    param_name: "one of --stdin, --stdout, --stderr".into(),
                    span: call.head,
                });
            }
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
