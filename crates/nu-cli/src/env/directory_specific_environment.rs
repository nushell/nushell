use indexmap::IndexMap;
use nu_protocol::{Primitive, UntaggedValue, Value};
use std::io::Write;
use std::{ffi::OsString, fmt::Debug, path::PathBuf, fs::OpenOptions};

#[derive(Debug, Default)]
pub struct DirectorySpecificEnvironment {
    pub whitelisted_directories: Vec<PathBuf>,

    //Directory -> Env key. If an environment var has been added from a .nu in a directory, we track it here so we can remove it when the user leaves the directory.
    pub added_env_vars: IndexMap<PathBuf, Vec<String>>,

    //Directory -> (env_key, value). If a .nu overwrites some existing environment variables, they are added here so that they can be restored later.
    pub overwritten_env_values: IndexMap<PathBuf, Vec<(String, OsString)>>,
}

impl DirectorySpecificEnvironment {
    pub fn new(whitelisted_directories: Option<Value>) -> DirectorySpecificEnvironment {
        let mut whitelisted_directories = if let Some(Value {
            value: UntaggedValue::Table(ref wrapped_directories),
            tag: _,
        }) = whitelisted_directories
        {
            wrapped_directories
                .iter()
                .fold(vec![], |mut directories, dirval| {
                    if let Value {
                        value: UntaggedValue::Primitive(Primitive::String(ref dir)),
                        tag: _,
                    } = dirval
                    {
                        directories.push(PathBuf::from(&dir));
                    }
                    directories
                })
        } else {
            vec![]
        };
        whitelisted_directories.sort();

        DirectorySpecificEnvironment {
            whitelisted_directories,
            added_env_vars: IndexMap::new(),
            overwritten_env_values: IndexMap::new(),
        }
    }

    // pub overwritten_env_values: IndexMap<PathBuf, Vec<(String, OsString)>>,
    //overwritten_env_values maps a directory with a .nu file to some environment variables that it overwrote.
    //if we are not in that directory, we re-add those variables.
    pub fn overwritten_values_to_restore(&mut self) -> std::io::Result<IndexMap<String, String>> {
        let current_dir = std::env::current_dir()?;

        let mut keyvals_to_restore = IndexMap::new();
        for (directory, keyvals) in &self.overwritten_env_values {
            let mut working_dir = Some(current_dir.as_path());

            let mut readd = true;
            while let Some(wdir) = working_dir {
                if wdir == directory.as_path() {
                    readd = false;
                    break;
                } else {
                    working_dir = working_dir.unwrap().parent();
                }
            }
            if readd {
                for (k, v) in keyvals {
                    keyvals_to_restore.insert(k.clone(), v.to_str().unwrap().to_string());
                }
            }
        }

        Ok(keyvals_to_restore)
    }

    pub fn env_vars_to_add(&mut self) -> std::io::Result<IndexMap<String, String>> {
        let current_dir = std::env::current_dir()?;

        let mut vars_to_add = IndexMap::new();
        for dir in &self.whitelisted_directories {
            //Start in the current directory, then traverse towards the root with working_dir to see if we are in a subdirectory of a valid directory.
            let mut working_dir = Some(current_dir.as_path());

            while let Some(wdir) = working_dir {
                if wdir == dir.as_path() {
                    //Read the .nu file and parse it into a nice map
                    let toml_doc = std::fs::read_to_string(wdir.join(".nu").as_path())?
                        .parse::<toml::Value>()
                        .unwrap();
                    let vars_in_current_file = toml_doc.get("env").unwrap().as_table().unwrap();

                    let keys_in_file: Vec<String> = vars_in_current_file
                        .iter()
                        .map(|(k, v)| {
                            vars_to_add.insert(k.clone(), v.as_str().unwrap().to_string()); //This is to add the keys and values to the environment
                            k.clone() //We add them all to a list to keep track of which keys we have added
                        })
                        .collect();

                    self.overwritten_env_values.insert(
                        wdir.to_path_buf(),
                        keys_in_file.iter().fold(vec![], |mut keyvals, key| {
                            if let Some(val) = std::env::var_os(key) {

                                let mut file = OpenOptions::new()
                                    .write(true)
                                    .append(true)
                                    .create(true)
                                    .open("restore.txt").unwrap();

                                write!(&mut file, "about to overwrite: {:?}\n", val).unwrap();

                                keyvals.push((key.clone(), val));
                                keyvals
                            } else {
                                keyvals
                            }
                        }),
                    );

                    self.added_env_vars.insert(wdir.to_path_buf(), keys_in_file);

                    break;
                } else {
                    working_dir = working_dir.unwrap().parent();
                }
            }
        }
        Ok(vars_to_add)
    }

    //If the user has left directories which added env vars through .nu, we clear those vars
    pub fn env_vars_to_delete(&mut self) -> std::io::Result<Vec<String>> {
        let current_dir = std::env::current_dir()?;

        //Gather up all environment variables that should be deleted.
        //If we are not in a directory or one of its subdirectories, mark the env_vals it maps to for removal.
        let vars_to_delete = self.added_env_vars.iter().fold(
            Vec::new(),
            |mut vars_to_delete, (directory, env_vars)| {
                let mut working_dir = Some(current_dir.as_path());

                while let Some(wdir) = working_dir {
                    if &wdir == directory {
                        return vars_to_delete;
                    } else {
                        working_dir = working_dir.unwrap().parent();
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
