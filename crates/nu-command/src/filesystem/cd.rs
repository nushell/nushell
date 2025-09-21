use std::path::PathBuf;

use nu_engine::command_prelude::*;
use nu_protocol::shell_error::{self, io::IoError};
use nu_utils::filesystem::{PermissionResult, have_permission};

#[derive(Clone)]
pub struct Cd;

impl Command for Cd {
    fn name(&self) -> &str {
        "cd"
    }

    fn description(&self) -> &str {
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

        // If getting PWD failed, default to the home directory. The user can
        // use `cd` to reset PWD to a good state.
        let cwd = engine_state
            .cwd(Some(stack))
            .ok()
            .or_else(nu_path::home_dir)
            .map(|path| path.into_std_path_buf())
            .unwrap_or_default();

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
                                return Err(shell_error::io::IoError::new(
                                    shell_error::io::ErrorKind::from_std(
                                        std::io::ErrorKind::NotADirectory,
                                    ),
                                    v.span,
                                    None,
                                )
                                .into());
                            };
                            path
                        } else {
                            return Err(shell_error::io::IoError::new(
                                ErrorKind::DirectoryNotFound,
                                v.span,
                                PathBuf::from(path_no_whitespace),
                            )
                            .into());
                        }
                    } else {
                        let path = nu_path::expand_path_with(path_no_whitespace, &cwd, true);
                        if !path.exists() {
                            return Err(shell_error::io::IoError::new(
                                ErrorKind::DirectoryNotFound,
                                v.span,
                                PathBuf::from(path_no_whitespace),
                            )
                            .into());
                        };
                        if !path.is_dir() {
                            return Err(shell_error::io::IoError::new(
                                shell_error::io::ErrorKind::from_std(
                                    std::io::ErrorKind::NotADirectory,
                                ),
                                v.span,
                                path,
                            )
                            .into());
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
            stack.add_env_var("OLDPWD".into(), oldpwd.clone())
        }

        match have_permission(&path) {
            //FIXME: this only changes the current scope, but instead this environment variable
            //should probably be a block that loads the information from the state in the overlay
            PermissionResult::PermissionOk => {
                stack.set_cwd(path)?;
                Ok(PipelineData::empty())
            }
            PermissionResult::PermissionDenied => Err(IoError::new(
                shell_error::io::ErrorKind::from_std(std::io::ErrorKind::PermissionDenied),
                call.head,
                path,
            )
            .into()),
        }
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Change to your home directory",
                example: r#"cd ~"#,
                result: None,
            },
            Example {
                description: r#"Change to the previous working directory (same as "cd $env.OLDPWD")"#,
                example: r#"cd -"#,
                result: None,
            },
            Example {
                description: "Changing directory with a custom command requires 'def --env'",
                example: r#"def --env gohome [] { cd ~ }"#,
                result: None,
            },
            Example {
                description: "Move two directories up in the tree (the parent directory's parent). Additional dots can be added for additional levels.",
                example: r#"cd ..."#,
                result: None,
            },
            Example {
                description: "The cd command itself is often optional. Simply entering a path to a directory will cd to it.",
                example: r#"/home"#,
                result: None,
            },
        ]
    }
}
