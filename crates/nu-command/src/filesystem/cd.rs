use nu_cmd_base::util::get_init_cwd;
use nu_engine::command_prelude::*;
use nu_utils::filesystem::{have_permission, PermissionResult};

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
        vec!["change", "directory", "dir", "folder", "switch"]
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("cd")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .switch("physical", "use the physical directory structure; resolve symbolic links before processing instances of ..", Some('P'))
            .optional("path", SyntaxShape::Directory, "The path to change to.")
            .input_output_types(vec![
                (Type::Nothing, Type::Nothing),
                (Type::String, Type::Nothing),
            ])
            .allow_variants_without_examples(true)
            .category(Category::FileSystem)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let physical = call.has_flag(engine_state, stack, "physical")?;
        let path_val: Option<Spanned<String>> = call.opt(engine_state, stack, 0)?;

        // If getting PWD failed, default to the initial directory. This way, the
        // user can use `cd` to recover PWD to a good state.
        let cwd = engine_state.cwd(Some(stack)).unwrap_or(get_init_cwd());

        let path_val = {
            if let Some(path) = path_val {
                Some(Spanned {
                    item: nu_utils::strip_ansi_string_unlikely(path.item),
                    span: path.span,
                })
            } else {
                path_val
            }
        };

        let path = match path_val {
            Some(v) => {
                if v.item == "-" {
                    if let Some(oldpwd) = stack.get_env_var(engine_state, "OLDPWD") {
                        oldpwd.to_path()?
                    } else {
                        cwd
                    }
                } else {
                    // Trim whitespace from the end of path.
                    let path_no_whitespace =
                        &v.item.trim_end_matches(|x| matches!(x, '\x09'..='\x0d'));

                    // If `--physical` is specified, canonicalize the path; otherwise expand the path.
                    if physical {
                        if let Ok(path) = nu_path::canonicalize_with(path_no_whitespace, &cwd) {
                            if !path.is_dir() {
                                return Err(ShellError::NotADirectory { span: v.span });
                            };
                            path
                        } else {
                            return Err(ShellError::DirectoryNotFound {
                                dir: path_no_whitespace.to_string(),
                                span: v.span,
                            });
                        }
                    } else {
                        let path = nu_path::expand_path_with(path_no_whitespace, &cwd, true);
                        if !path.exists() {
                            return Err(ShellError::DirectoryNotFound {
                                dir: path_no_whitespace.to_string(),
                                span: v.span,
                            });
                        };
                        if !path.is_dir() {
                            return Err(ShellError::NotADirectory { span: v.span });
                        };
                        path
                    }
                }
            }
            None => nu_path::expand_tilde("~"),
        };

        // Set OLDPWD.
        // We're using `Stack::get_env_var()` instead of `EngineState::cwd()` to avoid a conversion roundtrip.
        if let Some(oldpwd) = stack.get_env_var(engine_state, "PWD") {
            stack.add_env_var("OLDPWD".into(), oldpwd)
        }

        match have_permission(&path) {
            //FIXME: this only changes the current scope, but instead this environment variable
            //should probably be a block that loads the information from the state in the overlay
            PermissionResult::PermissionOk => {
                stack.set_cwd(path)?;
                Ok(PipelineData::empty())
            }
            PermissionResult::PermissionDenied(reason) => Err(ShellError::IOError {
                msg: format!(
                    "Cannot change directory to {}: {}",
                    path.to_string_lossy(),
                    reason
                ),
            }),
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Change to your home directory",
                example: r#"cd ~"#,
                result: None,
            },
            Example {
                description: "Change to the previous working directory ($OLDPWD)",
                example: r#"cd -"#,
                result: None,
            },
        ]
    }
}
