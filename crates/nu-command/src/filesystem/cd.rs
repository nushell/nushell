use nu_engine::{command_prelude::*, current_dir};
use std::path::Path;
#[cfg(unix)]
use {
    crate::filesystem::util::users,
    nix::{
        sys::stat::Mode,
        unistd::{Gid, Uid},
    },
    std::os::unix::fs::MetadataExt,
};

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
                        let path = oldpwd.to_path()?;
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
    match dir.as_ref().metadata() {
        Ok(metadata) => {
            let mode = Mode::from_bits_truncate(metadata.mode());
            let current_user_uid = users::get_current_uid();
            if current_user_uid.is_root() {
                return PermissionResult::PermissionOk;
            }
            let current_user_gid = users::get_current_gid();
            let owner_user = Uid::from_raw(metadata.uid());
            let owner_group = Gid::from_raw(metadata.gid());
            match (
                current_user_uid == owner_user,
                current_user_gid == owner_group,
            ) {
                (true, _) => {
                    if mode.contains(Mode::S_IXUSR) {
                        PermissionResult::PermissionOk
                    } else {
                        PermissionResult::PermissionDenied(
                            "You are the owner but do not have execute permission",
                        )
                    }
                }
                (false, true) => {
                    if mode.contains(Mode::S_IXGRP) {
                        PermissionResult::PermissionOk
                    } else {
                        PermissionResult::PermissionDenied(
                            "You are in the group but do not have execute permission",
                        )
                    }
                }
                (false, false) => {
                    if mode.contains(Mode::S_IXOTH)
                        || (mode.contains(Mode::S_IXGRP)
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

#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "android"))]
fn any_group(_current_user_gid: Gid, owner_group: Gid) -> bool {
    users::current_user_groups()
        .unwrap_or_default()
        .contains(&owner_group)
}

#[cfg(all(
    unix,
    not(any(target_os = "linux", target_os = "freebsd", target_os = "android"))
))]
fn any_group(current_user_gid: Gid, owner_group: Gid) -> bool {
    users::get_current_username()
        .and_then(|name| users::get_user_groups(&name, current_user_gid))
        .unwrap_or_default()
        .contains(&owner_group)
}
