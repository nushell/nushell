use super::{get_current_shell, get_last_shell, get_shells};
use nu_engine::{current_dir, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, PipelineData, ShellError, Signature, Spanned,
    SyntaxShape, Value,
};

/// Source a file for environment variables.
#[derive(Clone)]
pub struct GotoShell;

impl Command for GotoShell {
    fn name(&self) -> &str {
        "g"
    }

    fn signature(&self) -> Signature {
        Signature::build("g")
            .optional(
                "shell_number",
                SyntaxShape::String,
                "shell number to change to",
            )
            .category(Category::Shells)
    }

    fn usage(&self) -> &str {
        "Switch to a given shell, or list all shells if no given shell number."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;
        let new_shell: Option<Spanned<String>> = call.opt(engine_state, stack, 0)?;

        let cwd = current_dir(engine_state, stack)?;
        let cwd = Value::String {
            val: cwd.to_string_lossy().to_string(),
            span: call.head,
        };

        let shells = get_shells(engine_state, stack, cwd);

        match new_shell {
            Some(shell_span) => {
                let index = if shell_span.item == "-" {
                    get_last_shell(engine_state, stack)
                } else {
                    shell_span
                        .item
                        .parse::<usize>()
                        .map_err(|_| ShellError::NotFound(shell_span.span))?
                };

                let new_path = shells
                    .get(index)
                    .ok_or(ShellError::NotFound(shell_span.span))?
                    .to_owned();

                let current_shell = get_current_shell(engine_state, stack);

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
                        val: index as i64,
                        span: call.head,
                    },
                );
                stack.add_env_var(
                    "NUSHELL_LAST_SHELL".into(),
                    Value::Int {
                        val: current_shell as i64,
                        span: call.head,
                    },
                );

                stack.add_env_var("PWD".into(), new_path);

                Ok(PipelineData::new(call.head))
            }
            None => {
                let current_shell = get_current_shell(engine_state, stack);

                Ok(shells
                    .into_iter()
                    .enumerate()
                    .map(move |(idx, val)| Value::Record {
                        cols: vec!["active".to_string(), "path".to_string()],
                        vals: vec![
                            Value::Bool {
                                val: idx == current_shell,
                                span,
                            },
                            val,
                        ],
                        span,
                    })
                    .into_pipeline_data(None))
            }
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Lists all open shells",
                example: r#"g"#,
                result: None,
            },
            Example {
                description: "Make two directories and enter new shells for them, use `g` to jump to the specific shell",
                example: r#"mkdir foo bar; enter foo; enter ../bar; g 1"#,
                result: None,
            },
            Example {
                description: "Use `shells` to show all the opened shells and run `g 2` to jump to the third one",
                example: r#"shells; g 2"#,
                result: None,
            },
            Example {
                description: "Make two directories and enter new shells for them, use `g` to jump to the last used shell",
                example: r#"mkdir foo bar; enter foo; enter ../bar;  g 0; g -"#,
                result: None,
            },
        ]
    }
}
