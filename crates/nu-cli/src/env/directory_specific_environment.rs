use indexmap::IndexMap;
use nu_protocol::{Primitive, UntaggedValue, Value};
use std::{ffi::OsString, fmt::Debug, path::PathBuf};

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

    pub fn overwritten_values_to_restore(&mut self) -> std::io::Result<IndexMap<String, String>> {
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
                    working_dir = working_dir.unwrap().parent();
                }
            }
            if re_add_keyvals {
                for (k, v) in keyvals {
                    keyvals_to_restore.insert(k.clone(), v.to_str().unwrap().to_string());
                }
            }
        }

        self.overwritten_env_values = new_overwritten;
        Ok(keyvals_to_restore)
    }

    pub fn env_vars_to_add(&mut self) -> std::io::Result<IndexMap<String, String>> {
        let current_dir = std::env::current_dir()?;

        let mut vars_to_add = IndexMap::new();
        for dir in &self.whitelisted_directories {
            let mut working_dir = Some(current_dir.as_path());

            //Start in the current directory, then traverse towards the root with working_dir to see if we are in a subdirectory of a valid directory.
            while let Some(wdir) = working_dir {
                if wdir == dir.as_path() {
                    //Read the .nu file and parse it into a nice map
                    let toml_doc = std::fs::read_to_string(wdir.join(".nu").as_path())?
                        .parse::<toml::Value>()
                        .unwrap();
                    let vars_in_current_file = toml_doc.get("env").unwrap().as_table().unwrap();

                    let mut keys_in_current_nufile = vec![];
                    for (k, v) in vars_in_current_file {
                        vars_to_add.insert(k.clone(), v.as_str().unwrap().to_string()); //This is used to add variables to the environment
                        keys_in_current_nufile.push(k.clone()); //this is used to keep track of which directory added which variables
                    }

                    self.overwritten_env_values.insert(
                        //If we are about to overwrite any environment variables, we save them first so they can be restored later.
                        wdir.to_path_buf(),
                        keys_in_current_nufile
                            .iter()
                            .fold(vec![], |mut keyvals, key| {
                                if let Some(val) = std::env::var_os(key) {
                                    keyvals.push((key.clone(), val));
                                }
                                keyvals
                            }),
                    );

                    self.added_env_vars
                        .insert(wdir.to_path_buf(), keys_in_current_nufile);
                    break;
                } else {
                    working_dir = working_dir.unwrap().parent();
                }
            }
        }

        Ok(vars_to_add)
    }
}
