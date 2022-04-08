use nu_engine::{current_dir, CallExt};
use nu_protocol::ast::{Call, Expr, Expression};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Value};

#[derive(Clone)]
pub struct Cd;

impl Command for Cd {
    fn name(&self) -> &str {
        "cd"
    }

    fn usage(&self) -> &str {
        "Change directory."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("cd")
            .optional("path", SyntaxShape::Filepath, "the path to change to")
            .category(Category::FileSystem)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let raw_path = call.positional_nth(0);
        let path_val: Option<Value> = call.opt(engine_state, stack, 0)?;
        let cwd = current_dir(engine_state, stack)?;

        let (path, span) = match raw_path {
            Some(v) => match &v {
                Expression {
                    expr: Expr::Filepath(val),
                    span,
                    ..
                } if val == "-" => {
                    let oldpwd = stack.get_env_var(engine_state, "OLDPWD");

                    if let Some(oldpwd) = oldpwd {
                        let path = oldpwd.as_path()?;
                        let path = match nu_path::canonicalize_with(path, &cwd) {
                            Ok(p) => p,
                            Err(e) => {
                                return Err(ShellError::DirectoryNotFoundHelp(
                                    *span,
                                    format!("IO Error: {:?}", e),
                                ))
                            }
                        };
                        (path.to_string_lossy().to_string(), *span)
                    } else {
                        (cwd.to_string_lossy().to_string(), *span)
                    }
                }
                _ => match path_val {
                    Some(v) => {
                        let path = v.as_path()?;
                        let path = match nu_path::canonicalize_with(path, &cwd) {
                            Ok(p) => {
                                if !p.is_dir() {
                                    return Err(ShellError::NotADirectory(v.span()?));
                                }
                                p
                            }

                            Err(e) => {
                                return Err(ShellError::DirectoryNotFoundHelp(
                                    v.span()?,
                                    format!("IO Error: {:?}", e),
                                ))
                            }
                        };
                        (path.to_string_lossy().to_string(), v.span()?)
                    }
                    None => {
                        let path = nu_path::expand_tilde("~");
                        (path.to_string_lossy().to_string(), call.head)
                    }
                },
            },
            None => {
                let path = nu_path::expand_tilde("~");
                (path.to_string_lossy().to_string(), call.head)
            }
        };

        let path_value = Value::String { val: path, span };
        let cwd = Value::String {
            val: cwd.to_string_lossy().to_string(),
            span: call.head,
        };

        let shells = stack.get_env_var(engine_state, "NUSHELL_SHELLS");
        let mut shells = if let Some(v) = shells {
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

        shells[current_shell] = path_value.clone();

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

        if let Some(oldpwd) = stack.get_env_var(engine_state, "PWD") {
            stack.add_env_var("OLDPWD".into(), oldpwd)
        }

        //FIXME: this only changes the current scope, but instead this environment variable
        //should probably be a block that loads the information from the state in the overlay

        stack.add_env_var("PWD".into(), path_value);
        Ok(PipelineData::new(call.head))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Change to your home directory",
            example: r#"cd ~"#,
            result: None,
        }]
    }
}
