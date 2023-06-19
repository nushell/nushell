use crate::filesystem::cd_query::query;
use nu_engine::{current_dir, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Spanned, SyntaxShape, Type, Value,
};
use std::path::Path;

// The result of checking whether we have permission to cd to a directory
#[derive(Debug)]
enum PermissionResult {
    PermissionOk,
    PermissionDenied(String),
}

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
            .optional("path", SyntaxShape::Directory, "the path to change to")
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
        let path_val: Option<Spanned<String>> = call.opt(engine_state, stack, 0)?;
        let cwd = current_dir(engine_state, stack)?;
        let config = engine_state.get_config();
        let use_abbrev = config.cd_with_abbreviations;

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
                                                Some(format!("IO Error: {e:?}")),
                                            ))
                                        }
                                    }
                                } else {
                                    return Err(ShellError::DirectoryNotFound(
                                        v.span,
                                        Some(format!("IO Error: {e1:?}")),
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
                                                Some(format!("IO Error: {e:?}")),
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
                        Err(_) => {
                            if use_abbrev {
                                match query(&path_no_whitespace, None, v.span) {
                                    Ok(path) => path,
                                    Err(e) => {
                                        return Err(ShellError::DirectoryNotFound(
                                            v.span,
                                            Some(format!("IO Error: {e:?}")),
                                        ))
                                    }
                                }
                            } else {
                                return Err(ShellError::DirectoryNotFound(v.span, None));
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

        let path_value = Value::String {
            val: path.clone(),
            span,
        };

        if let Some(oldpwd) = stack.get_env_var(engine_state, "PWD") {
            stack.add_env_var("OLDPWD".into(), oldpwd)
        }

        match have_permission(&path) {
            //FIXME: this only changes the current scope, but instead this environment variable
            //should probably be a block that loads the information from the state in the overlay
            PermissionResult::PermissionOk => {
                stack.add_env_var("PWD".into(), path_value);
                Ok(PipelineData::empty())
            }
            PermissionResult::PermissionDenied(reason) => Err(ShellError::IOError(format!(
                "Cannot change directory to {path}: {reason}"
            ))),
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
                description: "Change to a directory via abbreviations",
                example: r#"cd d/s/9"#,
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

// TODO: Maybe we should use file_attributes() from https://doc.rust-lang.org/std/os/windows/fs/trait.MetadataExt.html
// More on that here: https://learn.microsoft.com/en-us/windows/win32/fileio/file-attribute-constants
#[cfg(windows)]
fn have_permission(dir: impl AsRef<Path>) -> PermissionResult {
    match dir.as_ref().read_dir() {
        Err(e) => {
            if matches!(e.kind(), std::io::ErrorKind::PermissionDenied) {
                PermissionResult::PermissionDenied("Folder is unable to be read".to_string())
            } else {
                PermissionResult::PermissionOk
            }
        }
        Ok(_) => PermissionResult::PermissionOk,
    }
}

// use exec mode to try to open a directory,
// same logic in https://github.com/fish-shell/fish-shell/blob/0cfdc9055132f2842007da95bf105a8d122141c3/src/fds.cpp#L237
#[cfg(unix)]
fn have_permission(dir: impl AsRef<Path>) -> PermissionResult {
    use nix::fcntl::OFlag;
    match nix::fcntl::open(
        dir.as_ref(),
        OFlag::O_CLOEXEC,
        nix::sys::stat::Mode::empty(),
    ) {
        Ok(_) => PermissionResult::PermissionOk,
        Err(e) => PermissionResult::PermissionDenied(e.to_string()),
    }
}
