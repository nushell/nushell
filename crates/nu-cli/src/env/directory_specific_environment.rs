use crate::commands::{self, autoenv::Trusted};
use commands::autoenv;
use indexmap::{IndexMap, IndexSet};
use nu_errors::ShellError;
use std::{
    collections::hash_map::DefaultHasher,
    ffi::OsString,
    fmt::Debug,
    hash::{Hash, Hasher},
    io::{Error, ErrorKind},
    path::{Path, PathBuf},
};

type EnvKey = String;
type EnvVal = OsString;
#[derive(Debug, Default)]
pub struct DirectorySpecificEnvironment {
    trusted: Option<Trusted>,

    //Directory -> Env key. If an environment var has been added from a .nu in a directory, we track it here so we can remove it when the user leaves the directory.
    added_env_vars: IndexMap<PathBuf, IndexSet<EnvKey>>,
}

impl DirectorySpecificEnvironment {
    pub fn new() -> DirectorySpecificEnvironment {
        let trusted = match autoenv::Trusted::read_trusted() {
            Ok(t) => Some(t),
            Err(_) => None,
        };
        DirectorySpecificEnvironment {
            trusted,
            added_env_vars: IndexMap::new(),
        }
    }

    fn toml_if_directory_is_trusted(&self, wdirenv: &PathBuf) -> Result<toml::Value, ShellError> {
        if let Some(trusted) = &self.trusted {
            let content = std::fs::read_to_string(&wdirenv)?;
            let mut hasher = DefaultHasher::new();
            content.hash(&mut hasher);

            if trusted.files.get(wdirenv.to_str().unwrap()) == Some(&hasher.finish().to_string()) {
                return Ok(content.parse::<toml::Value>().or_else(|_| {
                    Err(ShellError::untagged_runtime_error(
                        "Could not parse .nu-env file. Is it well-formed?",
                    ))
                })?);
            }
            return Err(ShellError::untagged_runtime_error(
                format!("{:?} is untrusted. Run 'autoenv trust {:?}' and restart nushell to trust it.\nThis needs to be done after each change to the file.", wdirenv, wdirenv.parent().unwrap_or_else(|| &Path::new("")))));
        }
        Err(ShellError::untagged_runtime_error("No trusted directories"))
    }

    pub fn env_vars_to_add(&mut self) -> Result<IndexMap<EnvKey, EnvVal>, ShellError> {
        let current_dir = std::env::current_dir()?;
        let mut working_dir = Some(current_dir.as_path());
        let mut vars_to_add = IndexMap::new();

        //Start in the current directory, then traverse towards the root with working_dir to see if we are in a subdirectory of a valid directory.
        while let Some(wdir) = working_dir {
            let wdirenv = wdir.join(".nu-env");
            if wdirenv.exists() {
                let toml_doc = self.toml_if_directory_is_trusted(&wdirenv)?;
                toml_doc
                    .get("env")
                    .ok_or_else(|| {
                        Error::new(
                            ErrorKind::InvalidData,
                            format!("[env] section missing in {:?}", wdirenv),
                        )
                    })?
                    .as_table()
                    .ok_or_else(|| {
                        Error::new(
                            ErrorKind::InvalidData,
                            format!("[env] section malformed in {:?}", wdirenv),
                        )
                    })?
                    .iter()
                    .for_each(|(dir_env_key, dir_env_val)| {
                        let dir_env_val: EnvVal = dir_env_val.as_str().unwrap().into();

                        //This condition is to make sure variables in parent directories don't overwrite variables set by subdirectories.
                        if !vars_to_add.contains_key(dir_env_key) {
                            vars_to_add.insert(dir_env_key.clone(), dir_env_val);

                            self.added_env_vars
                                .entry(wdir.to_path_buf())
                                .or_insert(IndexSet::new())
                                .insert(dir_env_key.clone());
                        }
                    });
            }

            working_dir = working_dir //Keep going up in the directory structure with .parent()
                .expect("This should not be None because of the while condition")
                .parent();
        }

        Ok(vars_to_add)
    }

    //If the user has left directories which added env vars through .nu, we clear those vars
    //once they are marked for deletion, remove them from added_env_vars
    pub fn env_vars_to_delete(&mut self) -> Result<IndexSet<EnvKey>, ShellError> {
        let current_dir = std::env::current_dir()?;
        let mut working_dir = Some(current_dir.as_path());

        //We start from the current directory and go towards the root. We retain the variables set by directories we are in.
        let mut new_added_env_vars = IndexMap::new();
        while let Some(wdir) = working_dir {
            if let Some(vars_added_by_this_directory) = self.added_env_vars.get(wdir) {
                //If we are still in a directory, we should continue to track the vars it added.
                new_added_env_vars.insert(wdir.to_path_buf(), vars_added_by_this_directory.clone());
            }
            working_dir = working_dir
                .expect("This should not be None because of the while condition")
                .parent();
        }

        // Gather up all environment variables that should be deleted.
        let mut vars_to_delete = IndexSet::new();
        for (dir, added_keys) in &self.added_env_vars {
            if !new_added_env_vars.contains_key(dir) {
                for k in added_keys {
                    vars_to_delete.insert(k.clone());
                }
            }
        }
        self.added_env_vars = new_added_env_vars;

        Ok(vars_to_delete)
    }
}
