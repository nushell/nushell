use indexmap::{IndexMap, IndexSet};
use nu_protocol::{Primitive, UntaggedValue, Value};
use std::{
    ffi::OsString,
    fmt::Debug,
    io::{Error, ErrorKind, Result},
    path::PathBuf,
};

type EnvKey = String;
type EnvVal = OsString;
#[derive(Debug, Default)]
pub struct DirectorySpecificEnvironment {
    allowed_directories: IndexSet<PathBuf>,

    //Directory -> Env key. If an environment var has been added from a .nu in a directory, we track it here so we can remove it when the user leaves the directory.
    added_env_vars: IndexMap<PathBuf, IndexSet<EnvKey>>,
}

impl DirectorySpecificEnvironment {
    pub fn new(allowed_directories: Option<Value>) -> DirectorySpecificEnvironment {
        let allowed_directories = if let Some(Value {
            value: UntaggedValue::Table(ref wrapped_directories),
            tag: _,
        }) = allowed_directories
        {
            wrapped_directories
                .iter()
                .filter_map(|dirval| {
                    if let Value {
                        value: UntaggedValue::Primitive(Primitive::String(ref dir)),
                        tag: _,
                    } = dirval
                    {
                        return Some(PathBuf::from(&dir));
                    }
                    None
                })
                .collect()
        } else {
            IndexSet::new()
        };

        DirectorySpecificEnvironment {
            allowed_directories,
            added_env_vars: IndexMap::new(),
        }
    }

    pub fn env_vars_to_add(&mut self) -> std::io::Result<IndexMap<EnvKey, EnvVal>> {
        let current_dir = std::env::current_dir()?;
        let mut working_dir = Some(current_dir.as_path());
        let mut vars_to_add = IndexMap::new();

        //Start in the current directory, then traverse towards the root with working_dir to see if we are in a subdirectory of a valid directory.
        while let Some(wdir) = working_dir {
            if self.allowed_directories.contains(wdir) {
                let toml_doc = std::fs::read_to_string(wdir.join(".nu-env").as_path())?
                    .parse::<toml::Value>()?;

                toml_doc
                    .get("env")
                    .ok_or_else(|| Error::new(ErrorKind::InvalidData, "env section missing"))?
                    .as_table()
                    .ok_or_else(|| Error::new(ErrorKind::InvalidData, "env section malformed"))?
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
    pub fn env_vars_to_delete(&mut self) -> Result<IndexSet<EnvKey>> {
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
