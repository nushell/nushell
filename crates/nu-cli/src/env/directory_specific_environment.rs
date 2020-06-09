use indexmap::{IndexMap, IndexSet};
use nu_protocol::{Primitive, UntaggedValue, Value};
use std::io::{Error, ErrorKind, Result};
use std::{ffi::OsString, fmt::Debug, path::PathBuf};

#[derive(Debug, Default)]
pub struct DirectorySpecificEnvironment {
    allowed_directories: IndexSet<PathBuf>,

    //Directory -> Env key. If an environment var has been added from a .nu in a directory, we track it here so we can remove it when the user leaves the directory.
    added_env_vars: IndexMap<PathBuf, Vec<String>>,

    //Directory -> (env_key, value). If a .nu overwrites some existing environment variables, they are added here so that they can be restored later.
    overwritten_env_values: IndexMap<PathBuf, Vec<(String, OsString)>>,
}

impl DirectorySpecificEnvironment {
    pub fn new(allowed_directories: Option<Value>) -> DirectorySpecificEnvironment {
        let mut allowed_directories = if let Some(Value {
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
            vec![]
        };
        allowed_directories.sort();
        let mut allowed = IndexSet::new();
        for d in allowed_directories {
            allowed.insert(d);
        }

        DirectorySpecificEnvironment {
            allowed_directories: allowed,
            added_env_vars: IndexMap::new(),
            overwritten_env_values: IndexMap::new(),
        }
    }

    //If we are no longer in a directory, we restore the values it overwrote.
    pub fn overwritten_values_to_restore(&mut self) -> Result<IndexMap<String, String>> {
        let current_dir = std::env::current_dir()?;

        let mut keyvals_to_restore = IndexMap::new();
        let mut new_overwritten = IndexMap::new();

        for (directory, keyvals) in &self.overwritten_env_values {
            let mut working_dir = Some(current_dir.as_path());

            let mut re_add_keyvals = true;
            while let Some(wdir) = working_dir {
                if wdir == directory.as_path() {
                    re_add_keyvals = false;
                    new_overwritten.insert(directory.clone(), keyvals.clone());
                    break;
                } else {
                    working_dir = working_dir //Keep going up in the directory structure with .parent()
                        .ok_or_else(|| {
                            Error::new(ErrorKind::NotFound, "Root directory has no parent")
                        })?
                        .parent();
                }
            }
            if re_add_keyvals {
                for (k, v) in keyvals {
                    keyvals_to_restore.insert(
                        k.clone(),
                        v.to_str()
                            .ok_or_else(|| {
                                Error::new(
                                    ErrorKind::Other,
                                    format!("{:?} is not valid unicode", v),
                                )
                            })?
                            .to_string(),
                    );
                }
            }
        }

        self.overwritten_env_values = new_overwritten;
        Ok(keyvals_to_restore)
    }

    pub fn env_vars_to_add(&mut self) -> Result<IndexMap<String, String>> {
        let current_dir = std::env::current_dir()?;
        let mut vars_to_add = IndexMap::new();
        let mut working_dir = Some(current_dir.as_path());

        //Start in the current directory, then traverse towards the root with working_dir to see if we are in a subdirectory of a valid directory.
        while let Some(wdir) = working_dir {
            if self.allowed_directories.contains(wdir) {
                let toml_doc = match std::fs::read_to_string(wdir.join(".nu").as_path()) {
                    Ok(doc) => doc.parse::<toml::Value>()?,
                    Err(_) => return Ok(vars_to_add),
                };

                let vars_in_current_file = toml_doc
                    .get("env")
                    .ok_or_else(|| {
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "No [env] section in .nu-file",
                        )
                    })?
                    .as_table()
                    .ok_or_else(|| {
                        Error::new(
                            ErrorKind::InvalidData,
                            "Malformed [env] section in .nu-file",
                        )
                    })?;

                let mut keys_in_current_nufile = vec![];
                for (k, v) in vars_in_current_file {
                    if !vars_to_add.contains_key(k) {
                        vars_to_add.insert(
                            k.clone(),
                            v.as_str()
                                .ok_or_else(|| {
                                    Error::new(
                                        ErrorKind::InvalidData,
                                        format!("Could not read environment variable: {}\n", v),
                                    )
                                })?
                                .to_string(),
                        ); //This is used to add variables to the environment
                    }
                    keys_in_current_nufile.push(k.clone()); //this is used to keep track of which directory added which variables
                }

                //If we are about to overwrite any environment variables, we save them first so they can be restored later.
                self.overwritten_env_values.insert(
                    wdir.to_path_buf(),
                    keys_in_current_nufile
                        .iter()
                        .filter_map(|key| {
                            if let Some(val) = std::env::var_os(key) {
                                return Some((key.clone(), val));
                            }
                            None
                        })
                        .collect(),
                );

                self.added_env_vars
                    .insert(wdir.to_path_buf(), keys_in_current_nufile);
            }
            working_dir =
                    working_dir //Keep going up in the directory structure with .parent()
                        .ok_or_else(|| {
                            Error::new(ErrorKind::NotFound, "Root directory has no parent")
                        })?
                        .parent();
        }

        Ok(vars_to_add)
    }

    //If the user has left directories which added env vars through .nu, we clear those vars
    pub fn env_vars_to_delete(&mut self) -> Result<Vec<String>> {
        let current_dir = std::env::current_dir()?;

        //Gather up all environment variables that should be deleted.
        //If we are not in a directory or one of its subdirectories, mark the env_vals it maps to for removal.
        let vars_to_delete = self.added_env_vars.iter().fold(
            Vec::new(),
            |mut vars_to_delete, (directory, env_vars)| {
                let mut working_dir = Some(current_dir.as_path());

                while let Some(wdir) = working_dir {
                    if wdir == directory {
                        return vars_to_delete;
                    } else {
                        working_dir = working_dir.expect("Root directory has no parent").parent();
                    }
                }
                //only delete vars from directories we are not in
                vars_to_delete.extend(env_vars.clone());
                vars_to_delete
            },
        );

        Ok(vars_to_delete)
    }
}
