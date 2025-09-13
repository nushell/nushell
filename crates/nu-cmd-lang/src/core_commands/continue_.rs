use nu_engine::command_prelude::*;
use nu_protocol::engine::CommandType;

#[derive(Clone)]
pub struct Continue;

impl Command for Continue {
    fn name(&self) -> &str {
        "continue"
    }

    fn description(&self) -> &str {
        "Continue a loop from the next iteration."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("continue")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .category(Category::Core)
    }

    fn extra_description(&self) -> &str {
        r#"This command is a parser keyword. For details, check:
  https://www.nushell.sh/book/thinking_in_nu.html

  continue can only be used in while, loop, and for loops. It can not be used with each or other filter commands"#
    }

    fn command_type(&self) -> CommandType {
        CommandType::Keyword
    }
    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        _call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // This is compiled specially by the IR compiler. The code here is never used when
        // running in IR mode.
        eprintln!(
            "Tried to execute 'run' for the 'continue' command: this code path should never be reached in IR mode"
        );
        unreachable!()
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Continue a loop from the next iteration",
            example: r#"for i in 1..10 { if $i == 5 { continue }; print $i }"#,
            result: None,
        }]
    }
}
