use crate::data::config::Conf;
use crate::env::directory_specific_environment::*;
use indexmap::{indexmap, IndexSet};
use nu_errors::ShellError;
use nu_protocol::{UntaggedValue, Value};
use std::env::*;
use std::ffi::OsString;

use std::fmt::Debug;

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
    pub autoenv: DirectorySpecificEnvironment,
}

impl Environment {
    pub fn new() -> Environment {
        Environment {
            environment_vars: None,
            path_vars: None,
            autoenv: DirectorySpecificEnvironment::new(),
        }
    }

    pub fn from_config<T: Conf>(configuration: &T) -> Environment {
        let env = configuration.env();
        let path = configuration.path();
        Environment {
            environment_vars: env,
            path_vars: path,
            autoenv: DirectorySpecificEnvironment::new(),
        }
    }

    pub fn autoenv(&mut self, reload_trusted: bool) -> Result<(), ShellError> {
        self.autoenv.maintain_autoenv()?;
        if reload_trusted {
            self.autoenv.clear_recently_untrusted_file()?;
        }
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

                let new_path_candidates = split_paths(&paths).map(|path| {
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
