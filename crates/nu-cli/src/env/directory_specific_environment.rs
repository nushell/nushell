use indexmap::{IndexMap, IndexSet};
use nu_command::commands::autoenv;
use nu_errors::ShellError;
use serde::Deserialize;
use std::env::*;
use std::process::Command;

use std::{
    ffi::OsString,
    fmt::Debug,
    path::{Path, PathBuf},
};

//Tests reside in /nushell/tests/shell/pipeline/commands/internal.rs

type EnvKey = String;
type EnvVal = OsString;
#[derive(Debug, Default)]
pub struct DirectorySpecificEnvironment {
    pub last_seen_directory: PathBuf,
    //If an environment var has been added from a .nu in a directory, we track it here so we can remove it when the user leaves the directory.
    //If setting the var overwrote some value, we save the old value in an option so we can restore it later.
    added_vars: IndexMap<PathBuf, IndexMap<EnvKey, Option<EnvVal>>>,

    //We track directories that we have read .nu-env from. This is different from the keys in added_vars since sometimes a file only wants to run scripts.
    visited_dirs: IndexSet<PathBuf>,
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
            added_vars: IndexMap::new(),
            visited_dirs: IndexSet::new(),
            exitscripts: IndexMap::new(),
        }
    }

    fn toml_if_trusted(&mut self, nu_env_file: &Path) -> Result<NuEnvDoc, ShellError> {
        let content = std::fs::read(&nu_env_file)?;

        if autoenv::file_is_trusted(&nu_env_file, &content)? {
            let mut doc: NuEnvDoc = toml::de::from_slice(&content)
                .map_err(|e| ShellError::untagged_runtime_error(format!("{:?}", e)))?;

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

    pub fn maintain_autoenv(&mut self) -> Result<(), ShellError> {
        let mut dir = current_dir()?;

        if self.last_seen_directory == dir {
            return Ok(());
        }

        //We track which keys we set as we go up the directory hierarchy, so that we don't overwrite a value we set in a subdir.
        let mut added_keys = IndexSet::new();

        let mut new_visited_dirs = IndexSet::new();
        let mut popped = true;
        while popped {
            let nu_env_file = dir.join(".nu-env");
            if nu_env_file.exists() && !self.visited_dirs.contains(&dir) {
                let nu_env_doc = self.toml_if_trusted(&nu_env_file)?;

                //add regular variables from the [env section]
                if let Some(env) = nu_env_doc.env {
                    for (env_key, env_val) in env {
                        self.maybe_add_key(&mut added_keys, &dir, &env_key, &env_val);
                    }
                }

                //Add variables that need to evaluate scripts to run, from [scriptvars] section
                if let Some(sv) = nu_env_doc.scriptvars {
                    for (key, script) in sv {
                        self.maybe_add_key(
                            &mut added_keys,
                            &dir,
                            &key,
                            value_from_script(&script)?.as_str(),
                        );
                    }
                }

                if let Some(es) = nu_env_doc.entryscripts {
                    for s in es {
                        run(s.as_str(), None)?;
                    }
                }

                if let Some(es) = nu_env_doc.exitscripts {
                    self.exitscripts.insert(dir.clone(), es);
                }
            }
            new_visited_dirs.insert(dir.clone());
            popped = dir.pop();
        }

        //Time to clear out vars set by directories that we have left.
        let mut new_vars = IndexMap::new();
        for (dir, dirmap) in self.added_vars.drain(..) {
            if new_visited_dirs.contains(&dir) {
                new_vars.insert(dir, dirmap);
            } else {
                for (k, v) in dirmap {
                    if let Some(v) = v {
                        std::env::set_var(k, v);
                    } else {
                        std::env::remove_var(k);
                    }
                }
            }
        }

        //Run exitscripts, can not be done in same loop as new vars as some files can contain only exitscripts
        let mut new_exitscripts = IndexMap::new();
        for (dir, scripts) in self.exitscripts.drain(..) {
            if new_visited_dirs.contains(&dir) {
                new_exitscripts.insert(dir, scripts);
            } else {
                for s in scripts {
                    run(s.as_str(), Some(&dir))?;
                }
            }
        }

        self.visited_dirs = new_visited_dirs;
        self.exitscripts = new_exitscripts;
        self.added_vars = new_vars;
        self.last_seen_directory = current_dir()?;
        Ok(())
    }

    pub fn maybe_add_key(
        &mut self,
        seen_vars: &mut IndexSet<EnvKey>,
        dir: &Path,
        key: &str,
        val: &str,
    ) {
        //This condition is to make sure variables in parent directories don't overwrite variables set by subdirectories.
        if !seen_vars.contains(key) {
            seen_vars.insert(key.to_string());
            self.added_vars
                .entry(PathBuf::from(dir))
                .or_insert(IndexMap::new())
                .insert(key.to_string(), var_os(key));

            std::env::set_var(key, val);
        }
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
            .added_vars
            .iter()
            .filter_map(|(path, _)| {
                if !current_trusted_files.contains(path) {
                    return Some(path.clone());
                }
                None
            })
            .collect();

        for path in untrusted_files {
            if let Some(added_keys) = self.added_vars.get(&path) {
                for (key, _) in added_keys {
                    remove_var(key);
                }
            }
            self.exitscripts.remove(&path);
            self.added_vars.remove(&path);
        }

        Ok(())
    }
}

fn run(cmd: &str, dir: Option<&PathBuf>) -> Result<(), ShellError> {
    if cfg!(target_os = "windows") {
        if let Some(dir) = dir {
            let command = format!("cd {} & {}", dir.to_string_lossy(), cmd);
            Command::new("cmd")
                .args(&["/C", command.as_str()])
                .output()?
        } else {
            Command::new("cmd").args(&["/C", cmd]).output()?
        }
    } else if let Some(dir) = dir {
        // FIXME: When nu scripting is added, cding like might not be a good idea. If nu decides to execute entryscripts when entering the dir this way, it will cause troubles.
        // For now only standard shell scripts are used, so this is an issue for the future.
        Command::new("sh")
            .arg("-c")
            .arg(format!("cd {:?}; {}", dir, cmd))
            .output()?
    } else {
        Command::new("sh").arg("-c").arg(&cmd).output()?
    };
    Ok(())
}
fn value_from_script(cmd: &str) -> Result<String, ShellError> {
    let command = if cfg!(target_os = "windows") {
        Command::new("cmd").args(&["/C", cmd]).output()?
    } else {
        Command::new("sh").arg("-c").arg(&cmd).output()?
    };
    if command.stdout.is_empty() {
        return Err(ShellError::untagged_runtime_error(format!(
            "{:?} did not return any output",
            cmd
        )));
    }
    let response = std::str::from_utf8(&command.stdout[..command.stdout.len()]).map_err(|e| {
        ShellError::untagged_runtime_error(format!(
            "Couldn't parse stdout from command {:?}: {:?}",
            command, e
        ))
    })?;

    Ok(response.trim().to_string())
}
