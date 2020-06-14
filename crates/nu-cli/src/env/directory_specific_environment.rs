use indexmap::{IndexMap, IndexSet};
use nu_protocol::{Primitive, UntaggedValue, Value};
use std::io::Write;
use std::io::{Error, ErrorKind, Result};
use std::{ffi::OsString, fmt::Debug, fs::OpenOptions, path::PathBuf};

#[derive(Debug, Default)]
pub struct DirectorySpecificEnvironment {
    allowed_directories: IndexSet<PathBuf>,

    //Directory -> Env key. If an environment var has been added from a .nu in a directory, we track it here so we can remove it when the user leaves the directory.
    added_env_vars: IndexMap<PathBuf, Vec<String>>,

    //Directory -> (env_key, value). If a .nu overwrites some existing environment variables, they are added here so that they can be restored later.
    overwritten_env_values: IndexMap<PathBuf, IndexMap<String, OsString>>,
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
            overwritten_env_values: IndexMap::new(),
        }
    }

    //If we are no longer in a directory, we restore the values it overwrote.
    pub fn overwritten_values_to_restore(&mut self) -> Result<IndexMap<String, String>> {
        let current_dir = std::env::current_dir()?;
        let mut working_dir = Some(current_dir.as_path());


        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open("restore.txt").unwrap();

        write!(&mut file, "about to restore: {:?}\n", self.overwritten_env_values).unwrap();

        let mut keyvals_to_restore = IndexMap::new();
        let mut new_overwritten_env_values = IndexMap::new();
        //If we are not in wdir or its subdir, remove its vals
        self.overwritten_env_values
            .iter()
            .for_each(|(directory, keyvals)| {
                while let Some(wdir) = working_dir {
                    if &wdir == directory {
                        keyvals.iter().for_each(|(k, v)| {
                            keyvals_to_restore.insert(k.clone(), v.to_str().unwrap().to_string());
                        });
                    }
                    working_dir = working_dir.expect("This directory has no parent").parent();
                }
                new_overwritten_env_values.insert(directory.clone(), keyvals.clone());
            });

        self.overwritten_env_values = new_overwritten_env_values;
        Ok(keyvals_to_restore)
    }

    pub fn env_vars_to_add(&mut self) -> Result<IndexMap<String, String>> {
        let current_dir = std::env::current_dir()?;
        let mut working_dir = Some(current_dir.as_path());

        let empty = toml::value::Table::new();
        let mut vars_to_add = IndexMap::new();

        //Start in the current directory, then traverse towards the root with working_dir to see if we are in a subdirectory of a valid directory.
        while let Some(wdir) = working_dir {
            if self.allowed_directories.contains(wdir) {
                let toml_doc = std::fs::read_to_string(wdir.join(".nu-env").as_path())
                    .unwrap_or_else(|_| r#"[env]"#.to_string())
                    .parse::<toml::Value>()?;

                toml_doc
                    .get("env")
                    .unwrap()
                    .as_table()
                    .unwrap_or_else(|| &empty)
                    .iter()
                    .for_each(|(k, v)| {
                        if !vars_to_add.contains_key(k) {
                            vars_to_add.insert(k.clone(), v.as_str().unwrap().to_string());

                            //If we are about to overwrite any environment variables, we save them first so they can be restored later.
                            if let Some(val) = std::env::var_os(k) {
                                self.overwritten_env_values
                                    .entry(wdir.to_path_buf())
                                    .or_insert(IndexMap::new())
                                    .insert(k.clone(), val);
                            } else {
                                self.added_env_vars
                                    .entry(wdir.to_path_buf())
                                    .or_insert(vec![])
                                    .push(k.clone());
                            }
                        }
                    });
            }

            let mut file = OpenOptions::new()
                .write(true)
                .append(true)
                .create(true)
                .open("restore.txt").unwrap();

            write!(&mut file, "overwritten: {:?}\n\n", self.overwritten_env_values).unwrap();

            working_dir = working_dir //Keep going up in the directory structure with .parent()
                .expect("This directory has no parent")
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
