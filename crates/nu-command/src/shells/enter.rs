use super::{get_current_shell, get_shells};
use nu_engine::{current_dir, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Value};

/// Source a file for environment variables.
#[derive(Clone)]
pub struct Enter;

impl Command for Enter {
    fn name(&self) -> &str {
        "enter"
    }

    fn signature(&self) -> Signature {
        Signature::build("enter")
            .required(
                "path",
                SyntaxShape::Filepath,
                "the path to enter as a new shell",
            )
            .category(Category::Shells)
    }

    fn usage(&self) -> &str {
        "Enters a new shell at the given path."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let new_path: Value = call.req(engine_state, stack, 0)?;
        let path_span = new_path.span()?;

        let new_path = new_path.as_path()?;

        let cwd = current_dir(engine_state, stack)?;
        let new_path = nu_path::canonicalize_with(new_path, &cwd)?;

        if !new_path.exists() {
            return Err(ShellError::DirectoryNotFound(path_span, None));
        }

        if !new_path.is_dir() {
            return Err(ShellError::DirectoryNotFoundCustom(
                "not a directory".to_string(),
                path_span,
            ));
        }

        let new_path = Value::String {
            val: new_path.to_string_lossy().to_string(),
            span: call.head,
        };

        let cwd = Value::String {
            val: cwd.to_string_lossy().to_string(),
            span: call.head,
        };

        let mut shells = get_shells(engine_state, stack, cwd);
        let mut current_shell = get_current_shell(engine_state, stack);

        stack.add_env_var(
            "NUSHELL_LAST_SHELL".into(),
            Value::Int {
                val: current_shell as i64,
                span: call.head,
            },
        );

        if current_shell + 1 > shells.len() {
            shells.push(new_path.clone());
            current_shell = shells.len();
        } else {
            shells.insert(current_shell + 1, new_path.clone());
            current_shell += 1;
        }

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
        vec![Example {
            description: "Enter a new shell at path '../dir-foo'",
            example: r#"enter ../dir-foo"#,
            result: None,
        }]
    }
}
