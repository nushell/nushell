use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, Type};

#[derive(Clone)]
pub struct Break;

impl Command for Break {
    fn name(&self) -> &str {
        "break"
    }

    fn usage(&self) -> &str {
        "Break a loop"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("break")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .category(Category::Core)
    }

    fn extra_usage(&self) -> &str {
        r#"This command is a parser keyword. For details, check:
  https://www.nushell.sh/book/thinking_in_nu.html"#
    }

    fn is_parser_keyword(&self) -> bool {
        true
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        Err(ShellError::Break(call.head))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Break out of a loop",
            example: r#"loop { break }"#,
            result: None,
        }]
    }
}
