use nu_engine::command_prelude::*;
use nu_protocol::engine::CommandType;

#[derive(Clone)]
pub struct Break;

impl Command for Break {
    fn name(&self) -> &str {
        "break"
    }

    fn usage(&self) -> &str {
        "Break a loop."
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

    fn command_type(&self) -> CommandType {
        CommandType::Keyword
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Err(ShellError::Break { span: call.head })
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Break out of a loop",
            example: r#"loop { break }"#,
            result: None,
        }]
    }
}
