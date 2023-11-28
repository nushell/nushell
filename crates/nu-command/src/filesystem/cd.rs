#[cfg(unix)]
use libc::gid_t;
use nu_engine::{current_dir, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Spanned, SyntaxShape, Type, Value,
};
use std::path::Path;

// For checking whether we have permission to cd to a directory
#[cfg(unix)]
mod file_permissions {
    pub type Mode = u32;
    pub const USER_EXECUTE: Mode = libc::S_IXUSR as Mode;
    pub const GROUP_EXECUTE: Mode = libc::S_IXGRP as Mode;
    pub const OTHER_EXECUTE: Mode = libc::S_IXOTH as Mode;
}

// The result of checking whether we have permission to cd to a directory
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
                            Err(_) => {
                                return Err(ShellError::DirectoryNotFound {
                                    dir: path.to_string_lossy().to_string(),
                                    span: v.span,
                                });
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
                                return Err(ShellError::NotADirectory { span: v.span });
                            };
                            p
                        }

                        // if canonicalize failed, let's check to see if it's abbreviated
                        Err(_) => {
                            return Err(ShellError::DirectoryNotFound {
                                dir: path_no_whitespace.to_string(),
                                span: v.span,
                            });
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

        let path_value = Value::string(path.clone(), span);

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
            PermissionResult::PermissionDenied(reason) => Err(ShellError::IOError {
                msg: format!("Cannot change directory to {path}: {reason}"),
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

// TODO: Maybe we should use file_attributes() from https://doc.rust-lang.org/std/os/windows/fs/trait.MetadataExt.html
// More on that here: https://learn.microsoft.com/en-us/windows/win32/fileio/file-attribute-constants
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
    use crate::filesystem::util::users;

    match dir.as_ref().metadata() {
        Ok(metadata) => {
            use std::os::unix::fs::MetadataExt;
            let bits = metadata.mode();
            let has_bit = |bit| bits & bit == bit;
            let current_user_uid = users::get_current_uid();
            if current_user_uid == 0 {
                return PermissionResult::PermissionOk;
            }
            let current_user_gid = users::get_current_gid();
            let owner_user = metadata.uid();
            let owner_group = metadata.gid();
            match (
                current_user_uid == owner_user,
                current_user_gid == owner_group,
            ) {
                (true, _) => {
                    if has_bit(file_permissions::USER_EXECUTE) {
                        PermissionResult::PermissionOk
                    } else {
                        PermissionResult::PermissionDenied(
                            "You are the owner but do not have execute permission",
                        )
                    }
                }
                (false, true) => {
                    if has_bit(file_permissions::GROUP_EXECUTE) {
                        PermissionResult::PermissionOk
                    } else {
                        PermissionResult::PermissionDenied(
                            "You are in the group but do not have execute permission",
                        )
                    }
                }
                (false, false) => {
                    if has_bit(file_permissions::OTHER_EXECUTE)
                        || (has_bit(file_permissions::GROUP_EXECUTE)
                            && any_group(current_user_gid, owner_group))
                    {
                        PermissionResult::PermissionOk
                    } else {
                        PermissionResult::PermissionDenied(
                            "You are neither the owner, in the group, nor the super user and do not have permission",
                        )
                    }
                }
            }
        }
        Err(_) => PermissionResult::PermissionDenied("Could not retrieve file metadata"),
    }
}

#[cfg(unix)]
fn any_group(current_user_gid: gid_t, owner_group: u32) -> bool {
    use crate::filesystem::util::users;

    users::get_current_username()
        .and_then(|name| users::get_user_groups(&name, current_user_gid))
        .unwrap_or_default()
        .into_iter()
        .any(|gid| gid.as_raw() == owner_group)
}
