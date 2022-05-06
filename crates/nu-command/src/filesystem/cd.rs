use crate::filesystem::cd_query::query;
use nu_engine::{current_dir, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Spanned, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct Cd;

impl Command for Cd {
    fn name(&self) -> &str {
        "cd"
    }

    fn usage(&self) -> &str {
        "Change directory."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["cd", "change", "directory", "dir", "folder", "switch"]
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("cd")
            .optional("path", SyntaxShape::Directory, "the path to change to")
            .category(Category::FileSystem)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let path_val: Option<Spanned<String>> = call.opt(engine_state, stack, 0)?;
        let cwd = current_dir(engine_state, stack)?;
        let config = engine_state.get_config();
        let use_abbrev = config.cd_with_abbreviations;

        let (path, span) = match path_val {
            Some(v) => {
                if v.item == "-" {
                    let oldpwd = stack.get_env_var(engine_state, "OLDPWD");

                    if let Some(oldpwd) = oldpwd {
                        let path = oldpwd.as_path()?;
                        let path = match nu_path::canonicalize_with(path.clone(), &cwd) {
                            Ok(p) => p,
                            Err(e1) => {
                                if use_abbrev {
                                    match query(&path, None, v.span) {
                                        Ok(p) => p,
                                        Err(e) => {
                                            return Err(ShellError::DirectoryNotFound(
                                                v.span,
                                                Some(format!("IO Error: {:?}", e)),
                                            ))
                                        }
                                    }
                                } else {
                                    return Err(ShellError::DirectoryNotFound(
                                        v.span,
                                        Some(format!("IO Error: {:?}", e1)),
                                    ));
                                }
                            }
                        };
                        (path.to_string_lossy().to_string(), v.span)
                    } else {
                        (cwd.to_string_lossy().to_string(), v.span)
                    }
                } else {
                    let path_no_whitespace =
                        &v.item.trim_end_matches(|x| matches!(x, '\x09'..='\x0d'));

                    let path = match nu_path::canonicalize_with(path_no_whitespace, &cwd) {
                        Ok(p) => {
                            if !p.is_dir() {
                                if use_abbrev {
                                    // if it's not a dir, let's check to see if it's something abbreviated
                                    match query(&p, None, v.span) {
                                        Ok(path) => path,
                                        Err(e) => {
                                            return Err(ShellError::DirectoryNotFound(
                                                v.span,
                                                Some(format!("IO Error: {:?}", e)),
                                            ))
                                        }
                                    };
                                } else {
                                    return Err(ShellError::NotADirectory(v.span));
                                }
                            };
                            p
                        }

                        // if canonicalize failed, let's check to see if it's abbreviated
                        Err(e1) => {
                            if use_abbrev {
                                match query(&path_no_whitespace, None, v.span) {
                                    Ok(path) => path,
                                    Err(e) => {
                                        return Err(ShellError::DirectoryNotFound(
                                            v.span,
                                            Some(format!("IO Error: {:?}", e)),
                                        ))
                                    }
                                }
                            } else {
                                return Err(ShellError::DirectoryNotFound(
                                    v.span,
                                    Some(format!("IO Error: {:?}", e1)),
                                ));
                            }
                        }
                    };
                    (path.to_string_lossy().to_string(), v.span)
                }
            }
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
        vec![
            Example {
                description: "Change to your home directory",
                example: r#"cd ~"#,
                result: None,
            },
            Example {
                description: "Change to a directory via abbreviations",
                example: r#"cd d/s/9"#,
                result: None,
            },
        ]
    }
}
