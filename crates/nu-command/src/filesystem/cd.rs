use crate::filesystem::cd_query::query;
use crate::{get_current_shell, get_shells};
use nu_engine::{current_dir, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Spanned, SyntaxShape, Value,
};
use std::path::Path;

// when the file under the fold executeable
#[cfg(unix)]
mod permission_mods {
    pub type Mode = u32;
    pub mod unix {
        use super::Mode;
        pub const USER_EXECUTE: Mode = libc::S_IXUSR as Mode;
        pub const GROUP_EXECUTE: Mode = libc::S_IXGRP as Mode;
        pub const OTHER_EXECUTE: Mode = libc::S_IXOTH as Mode;
    }
}

// use to return the message of the result of change director
// TODO: windows, maybe should use file_attributes function in https://doc.rust-lang.org/std/os/windows/fs/trait.MetadataExt.html
// TODO: the meaning of the result of the function can be found in https://learn.microsoft.com/en-us/windows/win32/fileio/file-attribute-constants
// TODO: if have realize the logic on windows, remove the cfg
#[derive(Debug)]
enum PermissionResult<'a> {
    PermissionOk,
    PermissionDenied(&'a str),
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

        let path_tointo = path.clone();
        let path_value = Value::String { val: path, span };
        let cwd = Value::String {
            val: cwd.to_string_lossy().to_string(),
            span: call.head,
        };

        let mut shells = get_shells(engine_state, stack, cwd);
        let current_shell = get_current_shell(engine_state, stack);
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
        match have_permission(&path_tointo) {
            PermissionResult::PermissionOk => {
                stack.add_env_var("PWD".into(), path_value);
                Ok(PipelineData::new(call.head))
            }
            PermissionResult::PermissionDenied(reason) => Err(ShellError::IOError(format!(
                "Cannot change directory to {}: {}",
                path_tointo, reason
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
#[cfg(windows)]
fn have_permission(dir: impl AsRef<Path>) -> PermissionResult<'static> {
    match dir.as_ref().read_dir() {
        Err(e) => {
            if matches!(e.kind(), std::io::ErrorKind::PermissionDenied) {
                PermissionResult::PermissionDenied("Folder is unable to be read")
            } else {
                PermissionResult::PermissionOk
            }
        }
        Ok(_) => PermissionResult::PermissionOk,
    }
}

#[cfg(unix)]
fn have_permission(dir: impl AsRef<Path>) -> PermissionResult<'static> {
    match dir.as_ref().metadata() {
        Ok(metadata) => {
            use std::os::unix::fs::MetadataExt;
            let bits = metadata.mode();
            let has_bit = |bit| bits & bit == bit;
            let current_user = users::get_current_uid();
            if current_user == 0 {
                return PermissionResult::PermissionOk;
            }
            let current_group = users::get_current_gid();
            let owner_user = metadata.uid();
            let owner_group = metadata.gid();
            match (current_user == owner_user, current_group == owner_group) {
                (true, _) => {
                    if has_bit(permission_mods::unix::USER_EXECUTE) {
                        PermissionResult::PermissionOk
                    } else {
                        PermissionResult::PermissionDenied(
                            "You are the owner but do not have the execute permission",
                        )
                    }
                }
                (false, true) => {
                    if has_bit(permission_mods::unix::GROUP_EXECUTE) {
                        PermissionResult::PermissionOk
                    } else {
                        PermissionResult::PermissionDenied(
                            "You are in the group but do not have the execute permission",
                        )
                    }
                }
                // other_user or root
                (false, false) => {
                    if has_bit(permission_mods::unix::OTHER_EXECUTE) {
                        PermissionResult::PermissionOk
                    } else {
                        PermissionResult::PermissionDenied(
                            "You are neither the owner, in the group, nor the super user and do not have permission",
                        )
                    }
                }
            }
        }
        Err(_) => PermissionResult::PermissionDenied("Could not retrieve the metadata"),
    }
}
