use crate::commands::{self, autoenv::Trusted};
use commands::autoenv;
use std::process::Command;
use indexmap::IndexMap;
use nu_errors::ShellError;
use serde::Deserialize;

use std::{
    ffi::OsString,
    fmt::Debug,
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

#[derive(Deserialize, Debug, Default)]
pub struct NuEnvDoc {
    pub env: IndexMap<String, String>,
    pub scriptvars: IndexMap<String, String>,
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

    fn toml_if_directory_is_trusted(&mut self, nu_env_file: &PathBuf) -> Result<NuEnvDoc, ShellError> {
        if let Some(trusted) = self.trusted.as_mut() {
            let content = std::fs::read(&nu_env_file)?;

            if trusted.file_is_trusted_reload_config(&nu_env_file, &content)?
            {
                let doc: NuEnvDoc = toml::de::from_str(std::str::from_utf8(&content).unwrap()).unwrap();
                return Ok(doc);
            }
            return Err(ShellError::untagged_runtime_error(
                format!("{:?} is untrusted. Run 'autoenv trust {:?}' and restart nushell to trust it.\nThis needs to be done after each change to the file.", nu_env_file, nu_env_file.parent().unwrap_or_else(|| &Path::new("")))));
        }
        Err(ShellError::untagged_runtime_error("No trusted directories"))
    }

    pub fn env_vars_to_add(&mut self) -> Result<IndexMap<EnvKey, EnvVal>, ShellError> {
        let mut working_dir = std::env::current_dir()?;
        let mut vars_to_add: IndexMap<EnvKey, EnvVal> = IndexMap::new();
        let nu_env_file = working_dir.join(".nu-env");

        //If we are in the last seen directory, do nothing
        //If we are in a parent directory to last_seen_directory, just return without applying .nu-env in the parent directory - they were already applied earlier.
        while self.last_seen_directory.cmp(&working_dir) == std::cmp::Ordering::Less { //parent.cmp(child) = Less
            if nu_env_file.exists() {
                let toml_doc = self.toml_if_directory_is_trusted(&nu_env_file)?;

                //add regular variables from the [env section]
                toml_doc
                    .env
                    .iter()
                    .for_each(|(dir_env_key, dir_env_val)| {
                        //This condition is to make sure variables in parent directories don't overwrite variables set by subdirectories.
                        if !vars_to_add.contains_key(dir_env_key) {
                            vars_to_add.insert(dir_env_key.clone(), OsString::from(dir_env_val));
                            self.added_env_vars
                                .entry(working_dir.clone())
                                .or_insert(IndexMap::new())
                                .insert(dir_env_key.clone(), std::env::var_os(dir_env_key));
                        }
                    });

                //Add variables that need to evaluate scripts to run
                toml_doc
                    .scriptvars
                    .iter()
                    .for_each(|(dir_env_key, dir_val_script)| {
                        let command = Command::new("sh")
                            .arg("-c")
                            .arg(dir_val_script)
                            .output()
                            .expect("couldn't exec");
                        let response = std::str::from_utf8(&command.stdout[..command.stdout.len() - 1]).ok();

                        if !vars_to_add.contains_key(dir_env_key) {
                            vars_to_add.insert(dir_env_key.clone(), OsString::from(response.unwrap().to_string()));
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
