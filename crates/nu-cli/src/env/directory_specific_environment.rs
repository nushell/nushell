use indexmap::{IndexMap, IndexSet};
use nu_protocol::{Primitive, UntaggedValue, Value};
use std::io::Write;
use std::{
    ffi::OsString,
    fmt::Debug,
    fs::OpenOptions,
    io::{Error, ErrorKind, Result},
    path::PathBuf,
};
use nu_errors::ShellError;

type EnvKey = String;
type EnvVal = OsString;
#[derive(Debug, Default)]
pub struct DirectorySpecificEnvironment {
    allowed_directories: IndexSet<PathBuf>,

    //Directory -> Env key. If an environment var has been added from a .nu in a directory, we track it here so we can remove it when the user leaves the directory.
    added_env_vars: IndexMap<PathBuf, IndexSet<EnvKey>>,

    //Directory -> (env_key, value). If a .nu overwrites some existing environment variables, they are added here so that they can be restored later.
    overwritten_env_vars: IndexMap<PathBuf, IndexMap<EnvKey, EnvVal>>,
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
            overwritten_env_vars: IndexMap::new(),
        }
    }

    //If we are no longer in a directory, we restore the values it overwrote.
    pub fn overwritten_values_to_restore(&mut self) -> Result<IndexMap<EnvKey, EnvVal>> {
        let current_dir = std::env::current_dir()?;
        let mut working_dir = Some(current_dir.as_path());

        let mut new_overwritten_env_values = IndexMap::new();
        let mut keyvals_to_restore = IndexMap::new();

        while let Some(wdir) = working_dir {
            if let Some(val) = self.overwritten_env_vars.get(wdir) {
                new_overwritten_env_values.insert(wdir.to_path_buf(), val.clone());
            }
            working_dir = working_dir
                .expect("This should not be None because of the while condition")
                .parent();
        }

        for (dir, keyvals) in &self.overwritten_env_vars {
            if !new_overwritten_env_values.contains_key(dir) {
                keyvals.iter().for_each(|(k, v)| {
                    keyvals_to_restore.insert(k.clone(), v.clone());
                });
            }
        }

        self.overwritten_env_vars = new_overwritten_env_values;
        Ok(keyvals_to_restore)
    }

    pub fn env_vars_to_add(&mut self) -> std::result::Result<IndexMap<EnvKey, EnvVal>, ShellError> {
        let current_dir = std::env::current_dir()?;
        let mut working_dir = Some(current_dir.as_path());

        let mut vars_to_add = IndexMap::new();

        // let mut file = OpenOptions::new()
        //     .write(true)
        //     .append(true)
        //     .create(true)
        //     .open("toadd.txt")
        //     .unwrap(
            // );

        // write!(&mut file, "1: {:?}\n", vars_to_add).unwrap();
        //WE CRASHING SOMEWHERE HERE

        //Start in the current directory, then traverse towards the root with working_dir to see if we are in a subdirectory of a valid directory.
        while let Some(wdir) = working_dir {
            if self.allowed_directories.contains(wdir) {
                let toml_doc = std::fs::read_to_string(wdir.join(".nu-env").as_path())?
                    .parse::<toml::Value>()?;

                toml_doc
                    .get("env")
                    .ok_or_else(|| Err(ShellError::untagged_runtime_error("env section missing")))?
                    .as_table()
                    .ok_or_else(|| Err(ShellError::untagged_runtime_error("env section malformed")))?
                    .iter()
                    .for_each(|(directory_env_key, directory_env_val)| {
                        if !vars_to_add.contains_key(directory_env_key) {
                            let directory_env_val: EnvVal =
                                directory_env_val.as_str().unwrap().into();

                            //If we are about to overwrite any environment variables, we save them first so they can be restored later.
                            if let Some(existing_val) = std::env::var_os(directory_env_key) {
                                if existing_val != directory_env_val {
                                    self.overwritten_env_vars
                                        .entry(wdir.to_path_buf())
                                        .or_insert(IndexMap::new())
                                        .insert(directory_env_key.clone(), existing_val);

                                    vars_to_add.insert(directory_env_key.clone(), directory_env_val);
                                }
                            } else {
                                //Otherwise, we just track that we added it here
                                self.added_env_vars
                                    .entry(wdir.to_path_buf())
                                    .or_insert(IndexSet::new())
                                    .insert(directory_env_key.clone());

                                vars_to_add.insert(directory_env_key.clone(), directory_env_val);
                            }
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
        let mut new_added_env_vars = IndexMap::new();
        let mut working_dir = Some(current_dir.as_path());

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
