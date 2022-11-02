use super::{get_current_shell, get_last_shell, get_shells};
use nu_engine::{current_dir, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Value};

#[derive(Clone)]
pub struct Exit;

impl Command for Exit {
    fn name(&self) -> &str {
        "exit"
    }

    fn signature(&self) -> Signature {
        Signature::build("exit")
            .optional(
                "exit_code",
                SyntaxShape::Int,
                "Exit code to return immediately with",
            )
            .switch(
                "now",
                "Exit out of all shells immediately (exiting Nu)",
                Some('n'),
            )
            .category(Category::Shells)
    }

    fn usage(&self) -> &str {
        "Exit a Nu shell or exit Nu entirely."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["quit", "close", "exit_code", "error_code", "logout"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let exit_code: Option<i64> = call.opt(engine_state, stack, 0)?;

        if let Some(exit_code) = exit_code {
            std::process::exit(exit_code as i32);
        }

        if call.has_flag("now") {
            std::process::exit(0);
        }

        let cwd = current_dir(engine_state, stack)?;
        let cwd = Value::String {
            val: cwd.to_string_lossy().to_string(),
            span: call.head,
        };

        let mut shells = get_shells(engine_state, stack, cwd);
        let mut current_shell = get_current_shell(engine_state, stack);
        let mut last_shell = get_last_shell(engine_state, stack);

        shells.remove(current_shell);

        if current_shell <= last_shell {
            last_shell = 0;
        }

        if current_shell == shells.len() && !shells.is_empty() {
            current_shell -= 1;
        }

        if shells.is_empty() {
            std::process::exit(0);
        } else {
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
            stack.add_env_var(
                "NUSHELL_LAST_SHELL".into(),
                Value::Int {
                    val: last_shell as i64,
                    span: call.head,
                },
            );

            stack.add_env_var("PWD".into(), new_path);

            Ok(PipelineData::new(call.head))
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Exit the current shell",
                example: "exit",
                result: None,
            },
            Example {
                description: "Exit all shells (exiting Nu)",
                example: "exit --now",
                result: None,
            },
        ]
    }
}
