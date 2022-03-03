use nu_engine::current_dir;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, PipelineData, ShellError, Signature, Value,
};

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
        let span = call.head;
        let cwd = current_dir(engine_state, stack)?;
        let cwd = Value::String {
            val: cwd.to_string_lossy().to_string(),
            span,
        };

        let shells = stack.get_env_var(engine_state, "NUSHELL_SHELLS");
        let shells = if let Some(v) = shells {
            v.as_list()
                .map(|x| x.to_vec())
                .unwrap_or_else(|_| vec![cwd])
        } else {
            vec![cwd]
        };

        let current_shell = stack.get_env_var(engine_state, "NUSHELL_CURRENT_SHELL");
        let current_shell = if let Some(v) = current_shell {
            v.as_integer().unwrap_or_default() as usize
        } else {
            0
        };

        let output = shells
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
            });

        Ok(output.into_pipeline_data(None))
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
