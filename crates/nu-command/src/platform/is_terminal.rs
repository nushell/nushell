use nu_protocol::{
    ast::{Argument, Call},
    engine::{Command, EngineState, Stack},
    Category, PipelineData, ShellError, Signature, Span, Type, Value,
};
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

    fn usage(&self) -> &str {
        "Check if stdin, stdout, or stderr is a terminal"
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let PipelineData::Empty = input else {
            let src_span = input.span().unwrap_or(Span::unknown());
            return Err(ShellError::PipelineMismatch { exp_input_type: "nothing".into(), dst_span: call.head, src_span  });
        };

        let stdin = call.has_flag("stdin");
        let stdout = call.has_flag("stdout");
        let stderr = call.has_flag("stderr");

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
                let span = match call.arguments.len() {
                    0 => call.span(),
                    1 => *argument_span(call.arguments.first().expect("at least one argument")),
                    _ => {
                        // Build a span covering all arguments.  At least one of them must be removed
                        let first =
                            argument_span(call.arguments.first().expect("at least one argument"));
                        let last =
                            argument_span(call.arguments.last().expect("at least one argument"));

                        Span::new(first.start, last.end)
                    }
                };

                return Err(ShellError::IncompatibleParametersSingle {
                    msg: "Only one stream may be checked".into(),
                    span,
                });
            }
        };

        Ok(PipelineData::Value(
            Value::bool(is_terminal, call.head),
            None,
        ))
    }
}

fn argument_span(argument: &Argument) -> &Span {
    match argument {
        Argument::Positional(e) => &e.span,
        Argument::Named((s, _, _)) => &s.span,
        Argument::Unknown(e) => &e.span,
    }
}
