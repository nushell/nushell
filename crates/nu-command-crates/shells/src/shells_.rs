use super::list_shells;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature};

/// Source a file for environment variables.
#[derive(Clone)]
pub struct Shells;

impl Command for Shells {
    fn name(&self) -> &str {
        "shells"
    }

    fn signature(&self) -> Signature {
        Signature::build("shells").category(Category::Shells)
    }

    fn usage(&self) -> &str {
        "Lists all open shells."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        list_shells(engine_state, stack, call.head)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Enter a new shell at parent path '..' and show all opened shells",
                example: r#"enter ..; shells"#,
                result: None,
            },
            Example {
                description: "Show currently active shell",
                example: r#"shells | where active == true"#,
                result: None,
            },
        ]
    }
}
