use crate::commands::{self, autoenv::Trusted};
use commands::autoenv;
use indexmap::{IndexMap};
use std::io::Write;
use nu_errors::ShellError;
use sha2::{Digest, Sha256};

use std::{
    ffi::OsString,
    fmt::Debug,
    path::{Path, PathBuf}, fs::OpenOptions,
};

type EnvKey = String;
type EnvVal = OsString;
#[derive(Debug, Default)]
pub struct DirectorySpecificEnvironment {
    trusted: Option<Trusted>,
    pub last_seen_directory: PathBuf,
    //Directory -> Env key. If an environment var has been added from a .nu in a directory, we track it here so we can remove it when the user leaves the directory.
    added_env_vars: IndexMap<PathBuf, IndexMap<EnvKey, Option<EnvVal>>>,
}

impl DirectorySpecificEnvironment {
    pub fn new() -> DirectorySpecificEnvironment {
        let trusted = match autoenv::Trusted::read_trusted() {
            Ok(t) => Some(t),
            Err(_) => None,
        };
        DirectorySpecificEnvironment {
            trusted,
            last_seen_directory: PathBuf::from("/"),
            added_env_vars: IndexMap::new(),
        }
    }

    fn toml_if_directory_is_trusted(&self, wdirenv: &PathBuf) -> Result<toml::Value, ShellError> {
        if let Some(trusted) = &self.trusted {
            let content = std::fs::read(&wdirenv)?;

            if trusted.files.get(wdirenv.to_str().unwrap_or(""))
                == Some(&Sha256::digest(&content).as_slice().to_vec())
            {
                return Ok(std::str::from_utf8(&content.as_slice()).or_else(|_| {
                    Err(ShellError::untagged_runtime_error(format!("Could not read {:?} as utf8 string", content)))
                })?
                .parse::<toml::Value>().or_else(|_| {
                    Err(ShellError::untagged_runtime_error(format!(
                        "Could not parse {:?}. Is it well-formed? Each entry must be written as key = \"value\" (note the quotation marks)",
                        wdirenv
                    )))
                })?);
            }
            return Err(ShellError::untagged_runtime_error(
                format!("{:?} is untrusted. Run 'autoenv trust {:?}' and restart nushell to trust it.\nThis needs to be done after each change to the file.", wdirenv, wdirenv.parent().unwrap_or_else(|| &Path::new("")))));
        }
        Err(ShellError::untagged_runtime_error("No trusted directories"))
    }

    pub fn env_vars_to_add(&mut self) -> Result<IndexMap<EnvKey, EnvVal>, ShellError> {
        let mut working_dir = std::env::current_dir()?;
        let mut vars_to_add = IndexMap::new();

        //If we are in the last seen directory, do nothing
        //If we are in a parent directory to last_seen_directory, just return without applying .nu-env in the parent directory - they were already applied earlier.
        //If current dir is parent to last_seen_directory, current.cmp(last) returns less
        //if current dir is the same as last_seen, current.cmp(last) returns equal

        if working_dir.cmp(&self.last_seen_directory) != std::cmp::Ordering::Greater {
            return Ok(vars_to_add);
        }
        while working_dir != self.last_seen_directory {
            let wdirenv = working_dir.join(".nu-env");
            if wdirenv.exists() {
                let toml_doc = self.toml_if_directory_is_trusted(&wdirenv)?;
                toml_doc
                    .get("env")
                    .ok_or_else(|| {
                        ShellError::untagged_runtime_error(format!(
                            "[env] section missing in {:?}",
                            wdirenv
                        ))
                    })?
                    .as_table()
                    .ok_or_else(|| {
                        ShellError::untagged_runtime_error(format!(
                            "[env] section malformed in {:?}",
                            wdirenv
                        ))
                    })?
                    .iter()
                    .for_each(|(dir_env_key, dir_env_val)| {
                        let dir_env_val: EnvVal = dir_env_val.as_str().unwrap_or("").into();

                        //This condition is to make sure variables in parent directories don't overwrite variables set by subdirectories.
                        if !vars_to_add.contains_key(dir_env_key) {
                            vars_to_add.insert(dir_env_key.clone(), dir_env_val);
                            self.added_env_vars
                                .entry(working_dir.clone())
                                .or_insert(IndexMap::new())
                                .insert(dir_env_key.clone(), std::env::var_os(dir_env_key));
                        }
                    });
            }
            working_dir.pop();
        }
        Ok(vars_to_add)
    }

    pub fn cleanup_after_dir_exit(
        &mut self,
    ) -> Result<IndexMap<EnvKey, Option<EnvVal>>, ShellError> {
        let current_dir = std::env::current_dir()?;
        let mut vars_to_cleanup = IndexMap::new();

        //If we are in the same directory as last_seen, or a subdirectory to it, do nothing
        //If we are in a subdirectory to last seen, do nothing
        //If we are in a parent directory to last seen, exit .nu-envs from last seen to parent and restore old vals
        if self.last_seen_directory.cmp(&current_dir) != std::cmp::Ordering::Greater {
            return Ok(vars_to_cleanup);
        }

        let mut working_dir = self.last_seen_directory.clone();

        while working_dir != current_dir {
            if let Some(vars_added_by_this_directory) = self.added_env_vars.get(&working_dir) {
                for (k, v) in vars_added_by_this_directory {
                    vars_to_cleanup.insert(k.clone(), v.clone());
                }

                self.added_env_vars.remove(&working_dir);
            }
            working_dir.pop();
        }

        Ok(vars_to_cleanup)
    }
}
