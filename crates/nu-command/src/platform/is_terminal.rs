use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, PipelineData, ShellError, Signature, Span, Type, Value,
};
use std::io::IsTerminal;

#[derive(Clone)]
pub struct Terminal;

impl Command for Terminal {
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
        engine_state: &EngineState,
        stack: &mut Stack,
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

        match (stdin, stdout, stderr) {
            (true, false, false) => Ok(PipelineData::Value(
                Value::bool(std::io::stdin().is_terminal(), call.head),
                None,
            )),
            (false, true, false) => Ok(PipelineData::Value(
                Value::bool(std::io::stdout().is_terminal(), call.head),
                None,
            )),
            (false, false, true) => Ok(PipelineData::Value(
                Value::bool(std::io::stderr().is_terminal(), call.head),
                None,
            )),
            (false, false, false) => Err(ShellError::MissingParameter {
                param_name: "one of --stdin, --stdout, --stderr".into(),
                span: call.head,
            }),
            (true, true, _) => {
                eprintln!("{:?}", call.arguments);

                Err(ShellError::IncompatibleParametersSingle {
                    msg: "Only one stream may be checked".into(),
                    span: call.head,
                })
            }
            (true, false, true) => todo!(),
            (false, true, true) => todo!(),
        }
    }
}
