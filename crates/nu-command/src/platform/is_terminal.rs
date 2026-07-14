use nu_engine::command_prelude::*;
use nu_protocol::OutDest;
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
        "Check if stdin, stdout, or stderr is a terminal."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Check if stdout is a terminal (default when no flag is specified).",
                example: "is-terminal",
                result: None,
            },
            Example {
                description: "Return false when output is piped to another command.",
                example: "is-terminal | to text",
                result: Some(Value::test_string("false")),
            },
            Example {
                description: "Return false when output is collected into a variable.",
                example: "let x = (is-terminal); $x",
                result: Some(Value::test_bool(false)),
            },
            Example {
                description: r#"Return "terminal attached" if standard input is attached to a terminal, and "no terminal" if not."#,
                example: r#"if (is-terminal --stdin) { "terminal attached" } else { "no terminal" }"#,
                result: Some(Value::test_string("terminal attached")),
            },
        ]
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
            (false, true, false) => stream_is_terminal(stack, Stream::Stdout),
            (false, false, true) => stream_is_terminal(stack, Stream::Stderr),
            (false, false, false) => stream_is_terminal(stack, Stream::Stdout),
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

enum Stream {
    Stdout,
    Stderr,
}

fn stream_is_terminal(stack: &Stack, stream: Stream) -> bool {
    let pipe_dest = match stream {
        Stream::Stdout => stack.pipe_stdout(),
        Stream::Stderr => stack.pipe_stderr(),
    };

    match pipe_dest {
        Some(
            OutDest::Pipe
            | OutDest::PipeSeparate
            | OutDest::Value
            | OutDest::Null
            | OutDest::File(_)
            | OutDest::Inherit,
        ) => false,
        Some(OutDest::Print) | None => match stream {
            Stream::Stdout => std::io::stdout().is_terminal(),
            Stream::Stderr => std::io::stderr().is_terminal(),
        },
    }
}
