use crate::data::config::Conf;
use indexmap::{indexmap, IndexSet};
use nu_protocol::{Primitive, UntaggedValue, Value};
use std::ffi::OsString;
use std::{collections::HashMap, fmt::Debug, fs::File, io::Read, path::PathBuf};

pub trait Env: Debug + Send {
    fn env(&self) -> Option<Value>;
    fn path(&self) -> Option<Value>;

    fn add_env(&mut self, key: &str, value: &str, overwrite_existing: bool);
    fn add_path(&mut self, new_path: OsString);
}

impl Env for Box<dyn Env> {
    fn env(&self) -> Option<Value> {
        (**self).env()
    }

    fn path(&self) -> Option<Value> {
        (**self).path()
    }

    fn add_env(&mut self, key: &str, value: &str, overwrite_existing: bool) {
        (**self).add_env(key, value, overwrite_existing);
    }

    fn add_path(&mut self, new_path: OsString) {
        (**self).add_path(new_path);
    }
}

#[derive(Debug, Default)]
struct DirectorySpecificEnvironment {
    pub whitelisted_directories: Vec<PathBuf>,
    pub added_env_vars: HashMap<PathBuf, Vec<String>>, //Directory -> Env key. If an environment var has been added from a .nu in a directory, we track it here so we can remove it when the user leaves the directory.
    pub overwritten_env_values: HashMap<PathBuf, Vec<(String, String)>>, //Directory -> (env_key, value). If a .nu overwrites some existing environment variables, they are added here so that they can be restored later.
}

impl DirectorySpecificEnvironment {
    pub fn new(whitelisted_directories: Vec<PathBuf>) -> DirectorySpecificEnvironment {
        DirectorySpecificEnvironment {
            whitelisted_directories,
            added_env_vars: HashMap::new(),
            overwritten_env_values: HashMap::new(),
        }
    }

    pub fn env_vars_to_add(&mut self) -> std::io::Result<HashMap<String, String>> {
        let current_dir = std::env::current_dir()?;

        let mut vars_to_add = HashMap::new();
        for dir in &self.whitelisted_directories {

            //Start in the current directory, then traverse towards the root directory with working_dir to check for .nu files
            let mut working_dir = Some(current_dir.as_path());

            while let Some(wdir) = working_dir {
                if wdir == dir.as_path() {
                    let mut dir = dir.clone();
                    dir.push(".nu");

                    //Read the .nu file and parse it into a nice map
                    let mut file = File::open(dir.as_path())?;
                    let mut contents = String::new();
                    file.read_to_string(&mut contents)?;
                    let toml_doc = contents.parse::<toml::Value>().unwrap();
                    let vars_in_current_file = toml_doc.get("env").unwrap().as_table().unwrap();

                    for (k, v) in vars_in_current_file {
                        vars_to_add.insert(k.clone(), v.as_str().unwrap().to_string());
                    }
                    break;
                } else {
                    working_dir = working_dir.unwrap().parent();
                }
            }
        }

        Ok(vars_to_add)
    }
}

#[derive(Debug, Default)]
pub struct Environment {
    environment_vars: Option<Value>,
    path_vars: Option<Value>,
    direnv: DirectorySpecificEnvironment,
}

impl Environment {
    pub fn new() -> Environment {
        Environment {
            environment_vars: None,
            path_vars: None,
            direnv: DirectorySpecificEnvironment::new(vec![]),
        }
    }


    pub fn from_config<T: Conf>(configuration: &T) -> Environment {
        let env = configuration.env();
        let path = configuration.path();

        let mut directories = vec![];
        if let Some(Value {
            value: UntaggedValue::Table(ref directories_as_values),
            tag: _,
        }) = configuration.direnv_whitelist()
        {
            for dirval in directories_as_values {
                if let Value {
                    value: UntaggedValue::Primitive(Primitive::String(ref dir)),
                    tag: _,
                } = dirval
                {
                    directories.push(PathBuf::from(&dir));
                }
            }
        };
        directories.sort();

        Environment {
            environment_vars: env,
            path_vars: path,
            direnv: DirectorySpecificEnvironment::new(directories),
        }
    }

    pub fn maintain_directory_environment(&mut self) -> std::io::Result<()> {
        let vars_to_add = self.direnv.env_vars_to_add()?;
        vars_to_add.iter().for_each(|(k, v)| {
            self.add_env(&k, &v, true);
        });
        Ok(())
    }

    pub fn morph<T: Conf>(&mut self, configuration: &T) {
        self.environment_vars = configuration.env();
        self.path_vars = configuration.path();
    }
}

impl Env for Environment {
    fn env(&self) -> Option<Value> {
        if let Some(vars) = &self.environment_vars {
            return Some(vars.clone());
        }

        None
    }

    fn path(&self) -> Option<Value> {
        if let Some(vars) = &self.path_vars {
            return Some(vars.clone());
        }

        None
    }

    fn add_env(&mut self, key: &str, value: &str, overwrite_existing: bool) {
        let value = UntaggedValue::string(value);

        let new_envs = {
            if let Some(Value {
                value: UntaggedValue::Row(ref envs),
                ref tag,
            }) = self.environment_vars
            {
                let mut new_envs = envs.clone();

                if !new_envs.contains_key(key) || overwrite_existing {
                    new_envs.insert_data_at_key(key, value.into_value(tag.clone()));
                }

                Value {
                    value: UntaggedValue::Row(new_envs),
                    tag: tag.clone(),
                }
            } else {
                UntaggedValue::Row(indexmap! { key.into() => value.into_untagged_value() }.into())
                    .into_untagged_value()
            }
        };

        self.environment_vars = Some(new_envs);
    }

    fn add_path(&mut self, paths: std::ffi::OsString) {
        let new_paths = {
            if let Some(Value {
                value: UntaggedValue::Table(ref current_paths),
                ref tag,
            }) = self.path_vars
            {
                let mut new_paths = current_paths.clone();

                let new_path_candidates = std::env::split_paths(&paths).map(|path| {
                    UntaggedValue::string(path.to_string_lossy()).into_value(tag.clone())
                });

                new_paths.extend(new_path_candidates);

                let paths: IndexSet<Value> = new_paths.into_iter().collect();

                Value {
                    value: UntaggedValue::Table(paths.into_iter().collect()),
                    tag: tag.clone(),
                }
            } else {
                let p = paths.into_string().unwrap_or_else(|_| String::from(""));
                let p = UntaggedValue::string(p).into_untagged_value();
                UntaggedValue::Table(vec![p]).into_untagged_value()
            }
        };

        self.path_vars = Some(new_paths);
    }
}

#[cfg(test)]
mod tests {
    use super::{Env, Environment};
    use crate::data::config::{tests::FakeConfig, Conf};
    use nu_protocol::UntaggedValue;
    use nu_test_support::fs::Stub::FileWithContent;
    use nu_test_support::playground::Playground;

    #[test]
    fn picks_up_environment_variables_from_configuration() {
        Playground::setup("environment_test_1", |dirs, sandbox| {
            sandbox.with_files(vec![FileWithContent(
                "configuration.toml",
                r#"
                    [env]
                    mosquetero_1 = "Andrés N. Robalino"
                    mosquetero_2 = "Jonathan Turner"
                    mosquetero_3 = "Yehuda katz"
                    mosquetero_4 = "Jason Gedge"
                "#,
            )]);

            let mut file = dirs.test().clone();
            file.push("configuration.toml");

            let fake_config = FakeConfig::new(&file);
            let actual = Environment::from_config(&fake_config);

            assert_eq!(actual.env(), fake_config.env());
        });
    }

    #[test]
    fn picks_up_path_variables_from_configuration() {
        Playground::setup("environment_test_2", |dirs, sandbox| {
            sandbox.with_files(vec![FileWithContent(
                "configuration.toml",
                r#"
                    path = ["/Users/andresrobalino/.volta/bin", "/users/mosqueteros/bin"]
                "#,
            )]);

            let mut file = dirs.test().clone();
            file.push("configuration.toml");

            let fake_config = FakeConfig::new(&file);
            let actual = Environment::from_config(&fake_config);

            assert_eq!(actual.path(), fake_config.path());
        });
    }

    #[test]
    fn updates_env_variable() {
        Playground::setup("environment_test_3", |dirs, sandbox| {
            sandbox.with_files(vec![FileWithContent(
                "configuration.toml",
                r#"
                    [env]
                    SHELL = "/usr/bin/you_already_made_the_nu_choice"
                "#,
            )]);

            let mut file = dirs.test().clone();
            file.push("configuration.toml");

            let fake_config = FakeConfig::new(&file);
            let mut actual = Environment::from_config(&fake_config);

            actual.add_env("USER", "NUNO");

            assert_eq!(
                actual.env(),
                Some(
                    UntaggedValue::row(
                        indexmap! {
                            "USER".into() => UntaggedValue::string("NUNO").into_untagged_value(),
                            "SHELL".into() => UntaggedValue::string("/usr/bin/you_already_made_the_nu_choice").into_untagged_value(),
                        }
                    ).into_untagged_value()
                )
            );
        });
    }

    #[test]
    fn does_not_update_env_variable_if_it_exists() {
        Playground::setup("environment_test_4", |dirs, sandbox| {
            sandbox.with_files(vec![FileWithContent(
                "configuration.toml",
                r#"
                    [env]
                    SHELL = "/usr/bin/you_already_made_the_nu_choice"
                "#,
            )]);

            let mut file = dirs.test().clone();
            file.push("configuration.toml");

            let fake_config = FakeConfig::new(&file);
            let mut actual = Environment::from_config(&fake_config);

            actual.add_env("SHELL", "/usr/bin/sh");

            assert_eq!(
                actual.env(),
                Some(
                    UntaggedValue::row(
                        indexmap! {
                            "SHELL".into() => UntaggedValue::string("/usr/bin/you_already_made_the_nu_choice").into_untagged_value(),
                        }
                    ).into_untagged_value()
                )
            );
        });
    }

    #[test]
    fn updates_path_variable() {
        Playground::setup("environment_test_5", |dirs, sandbox| {
            sandbox.with_files(vec![FileWithContent(
                "configuration.toml",
                r#"
                    path = ["/Users/andresrobalino/.volta/bin", "/users/mosqueteros/bin"]
                "#,
            )]);

            let mut file = dirs.test().clone();
            file.push("configuration.toml");

            let fake_config = FakeConfig::new(&file);
            let mut actual = Environment::from_config(&fake_config);

            actual.add_path(std::ffi::OsString::from("/path/to/be/added"));

            assert_eq!(
                actual.path(),
                Some(
                    UntaggedValue::table(&[
                        UntaggedValue::string("/Users/andresrobalino/.volta/bin")
                            .into_untagged_value(),
                        UntaggedValue::string("/users/mosqueteros/bin").into_untagged_value(),
                        UntaggedValue::string("/path/to/be/added").into_untagged_value(),
                    ])
                    .into_untagged_value()
                )
            );
        });
    }
}
