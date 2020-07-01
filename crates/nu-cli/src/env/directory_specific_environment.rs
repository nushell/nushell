use crate::commands::{self, autoenv::Trusted};
use commands::autoenv;
use indexmap::IndexMap;
use nu_errors::ShellError;
use serde::Deserialize;
use std::cmp::Ordering::Less;
use std::process::Command;

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
    exitscripts: IndexMap<PathBuf, Vec<String>>,
}

#[derive(Deserialize, Debug, Default)]
pub struct NuEnvDoc {
    pub env: IndexMap<String, String>,
    pub scriptvars: IndexMap<String, String>,
    pub entryscripts: Vec<String>,
    pub exitscripts: Vec<String>,
}

impl DirectorySpecificEnvironment {
    pub fn new() -> DirectorySpecificEnvironment {
        let root_dir = if cfg!(target_os = "windows") {
            PathBuf::from("c:\\")
        } else {
            PathBuf::from("/")
        };
        let trusted = autoenv::Trusted::read_trusted().ok();
        DirectorySpecificEnvironment {
            trusted,
            last_seen_directory: root_dir,
            added_env_vars: IndexMap::new(),
            exitscripts: IndexMap::new(),
        }
    }

    fn toml_if_directory_is_trusted(
        &mut self,
        nu_env_file: &PathBuf,
    ) -> Result<NuEnvDoc, ShellError> {
        if let Some(trusted) = self.trusted.as_mut() {
            let content = std::fs::read(&nu_env_file)?;

            if trusted.file_is_trusted_reload_config(&nu_env_file, &content)? {
                let doc: NuEnvDoc = toml::de::from_slice(&content)
                    .or_else(|e| Err(ShellError::untagged_runtime_error(format!("{:?}", e))))?;
                return Ok(doc);
            }
            return Err(ShellError::untagged_runtime_error(
                format!("{:?} is untrusted. Run 'autoenv trust {:?}' to trust it.\nThis needs to be done after each change to the file.", nu_env_file, nu_env_file.parent().unwrap_or_else(|| &Path::new("")))));
        }
        Err(ShellError::untagged_runtime_error("No trusted directories"))
    }

    pub fn add_key_if_appropriate(
        &mut self,
        vars_to_add: &mut IndexMap<EnvKey, EnvVal>,
        working_dir: &PathBuf,
        dir_env_key: &str,
        dir_env_val: &str,
    ) {
        //This condition is to make sure variables in parent directories don't overwrite variables set by subdirectories.
        if !vars_to_add.contains_key(dir_env_key) {
            vars_to_add.insert(dir_env_key.to_string(), OsString::from(dir_env_val));
            self.added_env_vars
                .entry(working_dir.clone())
                .or_insert(IndexMap::new())
                .insert(dir_env_key.to_string(), std::env::var_os(dir_env_key));
        }
    }

    pub fn env_vars_to_add(&mut self) -> Result<IndexMap<EnvKey, EnvVal>, ShellError> {
        let mut working_dir = std::env::current_dir()?;
        let mut vars_to_add: IndexMap<EnvKey, EnvVal> = IndexMap::new();
        let nu_env_file = working_dir.join(".nu-env");

        //If we are in the last seen directory, do nothing
        //If we are in a parent directory to last_seen_directory, just return without applying .nu-env in the parent directory - they were already applied earlier.
        //parent.cmp(child) = Less
        while self.last_seen_directory.cmp(&working_dir) == Less {
            if nu_env_file.exists() {
                let nu_env_doc = self.toml_if_directory_is_trusted(&nu_env_file)?;

                //add regular variables from the [env section]
                for (dir_env_key, dir_env_val) in nu_env_doc.env {
                    self.add_key_if_appropriate(
                        &mut vars_to_add,
                        &working_dir,
                        &dir_env_key,
                        &dir_env_val,
                    );
                }

                //Add variables that need to evaluate scripts to run, from [scriptvars] section
                for (dir_env_key, dir_val_script) in nu_env_doc.scriptvars {
                    let command = if cfg!(target_os = "windows") {
                        Command::new("cmd")
                            .args(&["/C", dir_val_script.as_str()])
                            .output()?
                    } else {
                        Command::new("sh").arg("-c").arg(dir_val_script).output()?
                    };
                    let response = std::str::from_utf8(&command.stdout[..command.stdout.len() - 1])
                        .or_else(|e| {
                            Err(ShellError::untagged_runtime_error(format!(
                                "Couldn't parse stdout from command {:?}: {:?}",
                                command, e
                            )))
                        })?;
                    self.add_key_if_appropriate(
                        &mut vars_to_add,
                        &working_dir,
                        &dir_env_key,
                        &response.to_string(),
                    );
                }

                for script in nu_env_doc.entryscripts {
                    if cfg!(target_os = "windows") {
                        Command::new("cmd")
                            .args(&["/C", script.as_str()])
                            .output()?;
                    } else {
                        Command::new("sh").arg("-c").arg(script).output()?;
                    }
                }
                self.exitscripts
                    .insert(working_dir.clone(), nu_env_doc.exitscripts);
            }

            // if true {
            //     return Ok(IndexMap::new());
            // }
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

        while current_dir.cmp(&working_dir) == Less {
            if let Some(vars_added_by_this_directory) = self.added_env_vars.get(&working_dir) {
                for (k, v) in vars_added_by_this_directory {
                    vars_to_cleanup.insert(k.clone(), v.clone());
                }
                self.added_env_vars.remove(&working_dir);
            }

            if let Some(scripts) = self.exitscripts.get(&working_dir) {
                for script in scripts {
                    if cfg!(target_os = "windows") {
                        Command::new("cmd")
                            .args(&["/C", script.as_str()])
                            .output()?;
                    } else {
                        Command::new("sh").arg("-c").arg(script).output()?;
                    }
                }
            }
            working_dir.pop();
        }
        Ok(vars_to_cleanup)
    }
}
