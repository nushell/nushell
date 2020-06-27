use crate::commands::{self, autoenv::Trusted};
use commands::autoenv;
use indexmap::IndexMap;
use nu_errors::ShellError;
use sha2::{Digest, Sha256};
use std::io::Write;

use std::{
    ffi::OsString,
    fmt::Debug,
    fs::OpenOptions,
    path::{Path, PathBuf},
};

type EnvKey = String;
type EnvVal = OsString;
#[derive(Debug, Default)]
pub struct DirectorySpecificEnvironment {
    trusted: Option<Trusted>,
    pub last_seen_directory: PathBuf,
    //If an environment var has been added from a .nu in a directory, we track it here so we can remove it when the user leaves the directory.
    //If setting the var overwrote some value, we save the old value in an option so we can restore it later.
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

    fn toml_if_directory_is_trusted(&self, nu_env_file: &PathBuf) -> Result<toml::Value, ShellError> {
        if let Some(trusted) = &self.trusted {
            let content = std::fs::read(&nu_env_file)?;

            if trusted.files.get(nu_env_file.to_str().unwrap_or(""))
                == Some(&Sha256::digest(&content).as_slice().to_vec())
            {
                return Ok(std::str::from_utf8(&content.as_slice()).or_else(|_| {
                    Err(ShellError::untagged_runtime_error(format!("Could not read {:?} as utf8 string", content)))
                })?
                .parse::<toml::Value>().or_else(|_| {
                    Err(ShellError::untagged_runtime_error(format!(
                        "Could not parse {:?}. Is it well-formed? Each entry must be written as key = \"value\" (note the quotation marks)",
                        nu_env_file
                    )))
                })?);
            }
            return Err(ShellError::untagged_runtime_error(
                format!("{:?} is untrusted. Run 'autoenv trust {:?}' and restart nushell to trust it.\nThis needs to be done after each change to the file.", nu_env_file, nu_env_file.parent().unwrap_or_else(|| &Path::new("")))));
        }
        Err(ShellError::untagged_runtime_error("No trusted directories"))
    }

    pub fn env_vars_to_add(&mut self) -> Result<IndexMap<EnvKey, EnvVal>, ShellError> {
        let mut working_dir = std::env::current_dir()?;
        let mut vars_to_add = IndexMap::new();
        let nu_env_file = working_dir.join(".nu-env");

        //If we are in the last seen directory, do nothing
        //If we are in a parent directory to last_seen_directory, just return without applying .nu-env in the parent directory - they were already applied earlier.
        //If current dir is parent to last_seen_directory, current.cmp(last) returns less
        //if current dir is the same as last_seen, current.cmp(last) returns equal
        while self.last_seen_directory.cmp(&working_dir) == std::cmp::Ordering::Less { //parent.cmp(child) = Less
            if nu_env_file.exists() {
                let toml_doc = self.toml_if_directory_is_trusted(&nu_env_file)?;
                toml_doc
                    .get("env")
                    .ok_or_else(|| {
                        ShellError::untagged_runtime_error(format!(
                            "[env] section missing in {:?}",
                            nu_env_file
                        ))
                    })?
                    .as_table()
                    .ok_or_else(|| {
                        ShellError::untagged_runtime_error(format!(
                            "[env] section malformed in {:?}",
                            nu_env_file
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
        let mut working_dir = self.last_seen_directory.clone();

        while current_dir.cmp(&working_dir) == std::cmp::Ordering::Less {
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
