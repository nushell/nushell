use crate::filesystem::cd_query::query;
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

// NOTE: it's a re-export of https://github.com/ogham/rust-users crate
// we need to use it because the upstream pr isn't merged: https://github.com/ogham/rust-users/pull/45
// once it's merged, we can remove this module
#[cfg(unix)]
mod nu_users {
    use libc::c_int;
    use std::ffi::{CString, OsStr};
    use std::os::unix::ffi::OsStrExt;
    use users::{gid_t, Group};
    /// Returns groups for a provided user name and primary group id.
    ///
    /// # libc functions used
    ///
    /// - [`getgrouplist`](https://docs.rs/libc/*/libc/fn.getgrouplist.html)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use users::get_user_groups;
    ///
    /// for group in get_user_groups("stevedore", 1001).expect("Error looking up groups") {
    ///     println!("User is a member of group #{} ({:?})", group.gid(), group.name());
    /// }
    /// ```
    pub fn get_user_groups<S: AsRef<OsStr> + ?Sized>(
        username: &S,
        gid: gid_t,
    ) -> Option<Vec<Group>> {
        // MacOS uses i32 instead of gid_t in getgrouplist for unknown reasons
        #[cfg(all(unix, target_os = "macos"))]
        let mut buff: Vec<i32> = vec![0; 1024];
        #[cfg(all(unix, not(target_os = "macos")))]
        let mut buff: Vec<gid_t> = vec![0; 1024];
        let name = CString::new(username.as_ref().as_bytes()).unwrap();
        let mut count = buff.len() as c_int;
        // MacOS uses i32 instead of gid_t in getgrouplist for unknown reasons
        #[cfg(all(unix, target_os = "macos"))]
        let res =
            unsafe { libc::getgrouplist(name.as_ptr(), gid as i32, buff.as_mut_ptr(), &mut count) };
        #[cfg(all(unix, not(target_os = "macos")))]
        let res = unsafe { libc::getgrouplist(name.as_ptr(), gid, buff.as_mut_ptr(), &mut count) };
        if res < 0 {
            None
        } else {
            buff.truncate(count as usize);
            buff.sort_unstable();
            buff.dedup();
            // allow trivial cast: on macos i is i32, on linux it's already gid_t
            #[allow(trivial_numeric_casts)]
            buff.into_iter()
                .filter_map(|i| users::get_group_by_gid(i as gid_t))
                .collect::<Vec<_>>()
                .into()
        }
    }
}

#[cfg(unix)]
fn any_group(current_user_gid: gid_t, owner_group: u32) -> bool {
    users::get_current_username()
        .map(|name| nu_users::get_user_groups(&name, current_user_gid).unwrap_or_default())
        .unwrap_or_default()
        .into_iter()
        .any(|group| group.gid() == owner_group)
}
