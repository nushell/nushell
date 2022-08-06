use super::{get_current_shell, get_shells};
use nu_engine::current_dir;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, Value};

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
        let cwd = current_dir(engine_state, stack)?;
        let cwd = Value::String {
            val: cwd.to_string_lossy().to_string(),
            span: call.head,
        };

        let shells = get_shells(engine_state, stack, cwd);
        let mut current_shell = get_current_shell(engine_state, stack);
        stack.add_env_var(
            "NUSHELL_LAST_SHELL".into(),
            Value::Int {
                val: current_shell as i64,
                span: call.head,
            },
        );

        current_shell += 1;

        if current_shell == shells.len() {
            current_shell = 0;
        }

        let new_path = shells[current_shell].clone();

        stack.add_env_var(
            "NUSHELL_SHELLS".into(),
            Value::List {
                vals: shells,
                span: call.head,
            },
        );
        stack.add_env_var(
            "NUSHELL_CURRENT_SHELL".into(),
            Value::Int {
                val: current_shell as i64,
                span: call.head,
            },
        );

        stack.add_env_var("PWD".into(), new_path);

        Ok(PipelineData::new(call.head))
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
