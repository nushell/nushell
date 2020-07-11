use crate::commands;
use commands::autoenv;
use indexmap::{IndexMap, IndexSet};
use nu_errors::ShellError;
use serde::Deserialize;
use std::cmp::Ordering::Less;
use std::env::*;
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
    pub last_seen_directory: PathBuf,
    //If an environment var has been added from a .nu in a directory, we track it here so we can remove it when the user leaves the directory.
    //If setting the var overwrote some value, we save the old value in an option so we can restore it later.
    added_env_vars: IndexMap<PathBuf, IndexMap<EnvKey, Option<EnvVal>>>,
    exitscripts: IndexMap<PathBuf, Vec<String>>,
}

#[derive(Deserialize, Debug, Default)]
pub struct NuEnvDoc {
    pub env: Option<IndexMap<String, String>>,
    pub scriptvars: Option<IndexMap<String, String>>,
    pub scripts: Option<IndexMap<String, Vec<String>>>,
    pub entryscripts: Option<Vec<String>>,
    pub exitscripts: Option<Vec<String>>,
}

impl DirectorySpecificEnvironment {
    pub fn new() -> DirectorySpecificEnvironment {
        let root_dir = if cfg!(target_os = "windows") {
            PathBuf::from("c:\\")
        } else {
            PathBuf::from("/")
        };
        DirectorySpecificEnvironment {
            last_seen_directory: root_dir,
            added_env_vars: IndexMap::new(),
            exitscripts: IndexMap::new(),
        }
    }

    fn toml_if_directory_is_trusted(
        &mut self,
        nu_env_file: &PathBuf,
    ) -> Result<NuEnvDoc, ShellError> {
        let content = std::fs::read(&nu_env_file)?;

        if autoenv::file_is_trusted(&nu_env_file, &content)? {
            let mut doc: NuEnvDoc = toml::de::from_slice(&content)
                .or_else(|e| Err(ShellError::untagged_runtime_error(format!("{:?}", e))))?;

            if let Some(scripts) = doc.scripts.as_ref() {
                for (k, v) in scripts {
                    if k == "entryscripts" {
                        doc.entryscripts = Some(v.clone());
                    } else if k == "exitscripts" {
                        doc.exitscripts = Some(v.clone());
                    }
                }
            }
            return Ok(doc);
        }
        Err(ShellError::untagged_runtime_error(
                format!("{:?} is untrusted. Run 'autoenv trust {:?}' to trust it.\nThis needs to be done after each change to the file.", nu_env_file, nu_env_file.parent().unwrap_or_else(|| &Path::new("")))))
    }

    pub fn env_vars_to_add(&mut self) -> Result<IndexMap<EnvKey, EnvVal>, ShellError> {
        let mut dir = current_dir()?;
        let mut vars_to_add: IndexMap<EnvKey, EnvVal> = IndexMap::new();
        if self.last_seen_directory == dir {
            return Ok(vars_to_add);
        }

        //If we are in the last seen directory, do nothing
        //If we are in a parent directory to last_seen_directory, just return without applying .nu-env in the parent directory - they were already applied earlier.
        //parent.cmp(child) = Less
        let mut popped = true;
        while self.last_seen_directory.cmp(&dir) != Greater && popped {
            let nu_env_file = dir.join(".nu-env");
            if nu_env_file.exists() {
                let nu_env_doc = self.toml_if_directory_is_trusted(&nu_env_file)?;

                //add regular variables from the [env section]
                if let Some(env) = nu_env_doc.env {
                    for (env_key, env_val) in env {
                        self.add_key_if_appropriate(&mut vars_to_add, &dir, &env_key, &env_val);
                    }
                }

                //Add variables that need to evaluate scripts to run, from [scriptvars] section
                if let Some(scriptvars) = nu_env_doc.scriptvars {
                    for (env_key, dir_val_script) in scriptvars {
                        let command = if cfg!(target_os = "windows") {
                            Command::new("cmd")
                                .args(&["/C", dir_val_script.as_str()])
                                .output()?
                        } else {
                            Command::new("sh").arg("-c").arg(&dir_val_script).output()?
                        };
                        if command.stdout.is_empty() {
                            return Err(ShellError::untagged_runtime_error(format!(
                                "{:?} in {:?} did not return any output",
                                dir_val_script, dir
                            )));
                        }
                        let response =
                            std::str::from_utf8(&command.stdout[..command.stdout.len() - 1])
                                .or_else(|e| {
                                    Err(ShellError::untagged_runtime_error(format!(
                                        "Couldn't parse stdout from command {:?}: {:?}",
                                        command, e
                                    )))
                                })?;
                        self.add_key_if_appropriate(
                            &mut vars_to_add,
                            &dir,
                            &env_key,
                            &response.to_string(),
                        );
                    }
                }

                if let Some(entryscripts) = nu_env_doc.entryscripts {
                    for script in entryscripts {
                        if cfg!(target_os = "windows") {
                            Command::new("cmd")
                                .args(&["/C", script.as_str()])
                                .output()?;
                        } else {
                            Command::new("sh").arg("-c").arg(script).output()?;
                        }
                    }
                }

                if let Some(exitscripts) = nu_env_doc.exitscripts {
                    self.exitscripts.insert(dir.clone(), exitscripts);
                }
            }
            popped = dir.pop();
        }

        Ok(vars_to_add)
    }

    pub fn add_key_if_appropriate(
        &mut self,
        vars_to_add: &mut IndexMap<EnvKey, EnvVal>,
        dir: &PathBuf,
        env_key: &str,
        env_val: &str,
    ) {
        //This condition is to make sure variables in parent directories don't overwrite variables set by subdirectories.
        if !vars_to_add.contains_key(env_key) {
            vars_to_add.insert(env_key.to_string(), OsString::from(env_val));
            self.added_env_vars
                .entry(dir.clone())
                .or_insert(IndexMap::new())
                .insert(env_key.to_string(), var_os(env_key));
        }
    }

    pub fn cleanup_after_dir_exit(
        &mut self,
    ) -> Result<IndexMap<EnvKey, Option<EnvVal>>, ShellError> {
        let current_dir = current_dir()?;
        let mut vars_to_cleanup = IndexMap::new();

        //If we are in the same directory as last_seen, or a subdirectory to it, do nothing
        //If we are in a subdirectory to last seen, do nothing
        //If we are in a parent directory to last seen, exit .nu-envs from last seen to parent and restore old vals
        let mut dir = self.last_seen_directory.clone();

        let mut popped = true;
        while current_dir.cmp(&dir) == Less && popped {
            if let Some(vars_added_by_this_directory) = self.added_env_vars.get(&dir) {
                for (k, v) in vars_added_by_this_directory {
                    vars_to_cleanup.insert(k.clone(), v.clone());
                }
                self.added_env_vars.remove(&dir);
            }

            if let Some(scripts) = self.exitscripts.get(&dir) {
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
            popped = dir.pop();
        }
        Ok(vars_to_cleanup)
    }

    // If the user recently ran autoenv untrust on a file, we clear the environment variables it set and make sure to not run any possible exitscripts.
    pub fn clear_recently_untrusted_file(&mut self) -> Result<(), ShellError> {
        // Figure out which file was untrusted
        // Remove all vars set by it
        let current_trusted_files: IndexSet<PathBuf> = autoenv::read_trusted()?
            .files
            .iter()
            .map(|(k, _)| PathBuf::from(k))
            .collect();

        // We figure out which file(s) the user untrusted by taking the set difference of current trusted files in .config/nu/nu-env.toml and the files tracked by self.added_env_vars
        // If a file is in self.added_env_vars but not in nu-env.toml, it was just untrusted.
        let untrusted_files: IndexSet<PathBuf> = self
            .added_env_vars
            .iter()
            .filter_map(|(path, _)| {
                if !current_trusted_files.contains(path) {
                    return Some(path.clone());
                }
                None
            })
            .collect();

        for path in untrusted_files {
            if let Some(added_keys) = self.added_env_vars.get(&path) {
                for (key, _) in added_keys {
                    remove_var(key);
                }
            }
            self.exitscripts.remove(&path);
            self.added_env_vars.remove(&path);
        }

        Ok(())
    }
}
