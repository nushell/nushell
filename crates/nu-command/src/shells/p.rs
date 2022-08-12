use super::{switch_shell, SwitchTo};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature};

/// Source a file for environment variables.
#[derive(Clone)]
pub struct PrevShell;

impl Command for PrevShell {
    fn name(&self) -> &str {
        "p"
    }

    fn signature(&self) -> Signature {
        Signature::build("p").category(Category::Shells)
    }

    fn usage(&self) -> &str {
        "Switch to the previous shell."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        switch_shell(engine_state, stack, call, call.head, SwitchTo::Prev)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Make two directories and enter new shells for them, use `p` to jump to the previous shell",
                example: r#"mkdir foo bar; enter foo; enter ../bar; p"#,
                result: None,
            },
            Example {
                description: "Run `p` several times and note the changes of current directory",
                example: r#"p"#,
                result: None,
            },
        ]
    }
}
