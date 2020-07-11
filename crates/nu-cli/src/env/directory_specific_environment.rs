use crate::commands;
use commands::autoenv;
use indexmap::{IndexMap, IndexSet};
use nu_errors::ShellError;
use serde::Deserialize;
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

    fn run_command(&self, cmd: &str) -> Result<(), ShellError>{
        if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(&["/C", cmd])
                .output()?
        } else {
            Command::new("sh").arg("-c").arg(&cmd).output()?
        };
        Ok(())
    }
    fn run_command_output(&self, cmd: &str) -> Result<std::process::Output, ShellError> {
        let command = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(&["/C", cmd])
                .output()?
        } else {
            Command::new("sh").arg("-c").arg(&cmd).output()?
        };
        if command.stdout.is_empty() {
            return Err(ShellError::untagged_runtime_error(format!(
                "{:?} did not return any output",
                cmd
            )));
        }
        Ok(command)
    }

    pub fn env_vars_to_add(&mut self) -> Result<(), ShellError> {
        let mut dir = current_dir()?;
        let mut added_keys = IndexSet::new();

        //Add all .nu-envs until we reach a dir which we have already added, or we reached the root.
        let mut popped = true;
        while !self.added_env_vars.contains_key(&dir) && popped {
            let nu_env_file = dir.join(".nu-env");
            if nu_env_file.exists() {
                let nu_env_doc = self.toml_if_directory_is_trusted(&nu_env_file)?;

                //add regular variables from the [env section]
                if let Some(env) = nu_env_doc.env {
                    for (env_key, env_val) in env {
                        self.add_key_if_appropriate(&mut added_keys, &dir, &env_key, &env_val);
                    }
                }

                //Add variables that need to evaluate scripts to run, from [scriptvars] section
                if let Some(scriptvars) = nu_env_doc.scriptvars {
                    for (env_key, dir_val_script) in scriptvars {
                        let command = self.run_command_output(&dir_val_script)?;
                        let response =
                            std::str::from_utf8(&command.stdout[..command.stdout.len() - 1])
                                .or_else(|e| {
                                    Err(ShellError::untagged_runtime_error(format!(
                                        "Couldn't parse stdout from command {:?}: {:?}",
                                        command, e
                                    )))
                                })?;
                        self.add_key_if_appropriate(
                            &mut added_keys,
                            &dir,
                            &env_key,
                            &response.to_string(),
                        );
                    }
                }

                if let Some(entryscripts) = nu_env_doc.entryscripts {
                    for script in entryscripts {
                        self.run_command(script.as_str())?;
                    }
                }

                if let Some(exitscripts) = nu_env_doc.exitscripts {
                    self.exitscripts.insert(dir.clone(), exitscripts);
                }
            }
            popped = dir.pop();
        }

        Ok(())
    }

    pub fn add_key_if_appropriate(
        &mut self,
        vars_to_add: &mut IndexSet<EnvKey>,
        dir: &PathBuf,
        env_key: &str,
        env_val: &str,
    ) {
        //This condition is to make sure variables in parent directories don't overwrite variables set by subdirectories.
        if !vars_to_add.contains(env_key) {
            vars_to_add.insert(env_key.to_string());
            self.added_env_vars
                .entry(dir.clone())
                .or_insert(IndexMap::new())
                .insert(env_key.to_string(), var_os(env_key));

            std::env::set_var(env_key, env_val);
        }
    }

    pub fn cleanup_after_dir_exit(&mut self) -> Result<(), ShellError> {
        let mut dir = current_dir()?;
        let mut seen_directories = IndexSet::new();
        let mut popped = true;

        //Go upward from current dir. Each directory we pass that is in added_env_vars we save, the rest we remove
        while popped {
            seen_directories.insert(dir.clone());

            popped = dir.pop();
        }

        let mut new_env_vars = IndexMap::new();
        for (dir, dirmap) in &self.added_env_vars {
            if seen_directories.contains(dir) {
                new_env_vars.insert(dir.clone(), dirmap.clone());
            } else {

                if let Some(scripts) = self.exitscripts.get(dir) {
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

                for (k, v) in dirmap {
                    if let Some(v) = v {
                        std::env::set_var(k, v);
                    } else {
                        std::env::remove_var(k);
                    }
                }
            }
        }
        self.added_env_vars = new_env_vars;
        Ok(())
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
