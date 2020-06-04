use crate::data::config::Conf;
use indexmap::{indexmap, IndexSet};
use nu_protocol::{UntaggedValue, Value};
use std::collections::{HashMap};
use std::ffi::OsString;
use std::fmt::Debug;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;

pub trait Env: Debug + Send {
    fn env(&self) -> Option<Value>;
    fn path(&self) -> Option<Value>;

    fn add_env(&mut self, key: &str, value: &str);
    fn add_path(&mut self, new_path: OsString);
}

impl Env for Box<dyn Env> {
    fn env(&self) -> Option<Value> {
        (**self).env()
    }

    fn path(&self) -> Option<Value> {
        (**self).path()
    }

    fn add_env(&mut self, key: &str, value: &str) {
        (**self).add_env(key, value);
    }

    fn add_path(&mut self, new_path: OsString) {
        (**self).add_path(new_path);
    }
}

#[derive(Debug, Default)]
pub struct Environment {
    environment_vars: Option<Value>,
    path_vars: Option<Value>,
    nurc_env_keys: HashMap<PathBuf, Vec<String>>, //Directory -> Env key. If an environment var has been added from a .nurc in a directory, we track it here so we can remove it when the user leaves the directory.
}

impl Environment {
    pub fn new() -> Environment {
        Environment {
            environment_vars: None,
            path_vars: None,
            nurc_env_keys: HashMap::new(),
        }
    }

    pub fn from_config<T: Conf>(configuration: &T) -> Environment {
        let env = configuration.env();
        let path = configuration.path();

        Environment {
            environment_vars: env,
            path_vars: path,
            nurc_env_keys: HashMap::new(),
        }
    }

    //Add env vars specified in the current dirs .nurc, if it exists.
    //TODO: Remove env vars after leaving the directory. Save added vars in env?
    //Map directory to vars
    //TODO: Add authentication by saving the path to the .nurc file in some variable?
    //TODO: handle errors

    pub fn maintain_nurc_environment_vars(&mut self) {
        match self.clear_vars_from_unvisited_dirs() {
            _ => {}
        };
        match self.add_nurc() {
            _ => {}
        };
    }

    pub fn add_nurc(&mut self) -> std::io::Result<()> {
        let mut file = File::open(".nurc")?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        let toml_doc = contents.parse::<toml::Value>().unwrap();
        let nurc_vars = toml_doc.get("env").unwrap().as_table().unwrap();

        nurc_vars.iter().for_each(|(k, v)| {
            self.add_env(k, v.as_str().unwrap());
        });

        self.nurc_env_keys.insert(
            std::env::current_dir()?,
            nurc_vars.keys().map(|k| k.clone()).collect(),
        ); //Maybe could do without clone here, but leave for now
        Ok(())
    }

    //If the user has left directories which added env vars through .nurc, we clear those vars
    //For each directory d in nurc_env_vars:
    //if current_dir does not have d as a parent (possibly recursive), the vars set by d should be removed
    pub fn clear_vars_from_unvisited_dirs(&mut self) -> std::io::Result<()> {
        let current_dir = std::env::current_dir()?;

        let mut new_nurc_env_vars = HashMap::new();
        for (d, v) in self.nurc_env_keys.iter() {
            let mut working_dir = Some(current_dir.as_path());
            while working_dir.is_some() {
                if working_dir.unwrap() == d {
                    new_nurc_env_vars.insert(d.clone(), v.clone());
                    break;
                } else {
                    working_dir = working_dir.unwrap().parent();
                }
            }
        }

        let mut vars_to_delete = vec![];
        for (path, vals) in self.nurc_env_keys.iter() {
            if !new_nurc_env_vars.contains_key(path) {
                vars_to_delete.extend(vals.clone());
            }
        }

        vars_to_delete.iter().for_each(|env_var| self.remove_env(env_var));

        self.nurc_env_keys = new_nurc_env_vars;
        Ok(())
    }

    pub fn remove_env(&mut self, key: &str) {
        if let Some(Value {
            value: UntaggedValue::Row(envs),
            tag: _,
        }) = &mut self.environment_vars
        {
            envs.remove_key(key);
        }
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

    fn add_env(&mut self, key: &str, value: &str) {
        let value = UntaggedValue::string(value);

        let new_envs = {
            if let Some(Value {
                value: UntaggedValue::Row(ref envs),
                ref tag,
            }) = self.environment_vars
            {
                let mut new_envs = envs.clone();

                if !new_envs.contains_key(key) {
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
