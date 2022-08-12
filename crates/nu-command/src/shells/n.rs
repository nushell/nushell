use super::{switch_shell, SwitchTo};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature};

/// Source a file for environment variables.
#[derive(Clone)]
pub struct NextShell;

impl Command for NextShell {
    fn name(&self) -> &str {
        "n"
    }

    fn signature(&self) -> Signature {
        Signature::build("n").category(Category::Shells)
    }

    fn usage(&self) -> &str {
        "Switch to the next shell."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        switch_shell(engine_state, stack, call, call.head, SwitchTo::Next)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Make two directories and enter new shells for them, use `n` to jump to the next shell",
                example: r#"mkdir foo bar; enter foo; enter ../bar; n"#,
                result: None,
            },
            Example {
                description: "Run `n` several times and note the changes of current directory",
                example: r#"n"#,
                result: None,
            },
        ]
    }
}
