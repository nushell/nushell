use nu_engine::{current_dir, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Spanned, SyntaxShape, Value,
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
            .required(
                "shell_number",
                SyntaxShape::Int,
                "shell number to change to",
            )
            .category(Category::Shells)
    }

    fn usage(&self) -> &str {
        "Switch to a given shell."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let new_shell: Spanned<i64> = call.req(engine_state, stack, 0)?;

        let cwd = current_dir(engine_state, stack)?;
        let cwd = Value::String {
            val: cwd.to_string_lossy().to_string(),
            span: call.head,
        };

        let shells = stack.get_env_var(engine_state, "NUSHELL_SHELLS");
        let shells = if let Some(v) = shells {
            v.as_list()
                .map(|x| x.to_vec())
                .unwrap_or_else(|_| vec![cwd])
        } else {
            vec![cwd]
        };

        let new_path = if let Some(v) = shells.get(new_shell.item as usize) {
            v.clone()
        } else {
            return Err(ShellError::NotFound(new_shell.span));
        };

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
                val: new_shell.item,
                span: call.head,
            },
        );

        stack.add_env_var("PWD".into(), new_path);

        Ok(PipelineData::new(call.head))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
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
        ]
    }
}
