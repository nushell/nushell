use crate::env::environment::Environment;
use nu_data::config::{Conf, NuConfig};
use nu_engine::Env;
use nu_engine::EvaluationContext;
use nu_errors::ShellError;
use parking_lot::Mutex;
use std::sync::{atomic::Ordering, Arc};

pub struct EnvironmentSyncer {
    pub env: Arc<Mutex<Box<Environment>>>,
    pub config: Arc<Mutex<Box<dyn Conf>>>,
}

impl Default for EnvironmentSyncer {
    fn default() -> Self {
        Self::new()
    }
}

impl EnvironmentSyncer {
    pub fn with_config(config: Box<dyn Conf>) -> Self {
        EnvironmentSyncer {
            env: Arc::new(Mutex::new(Box::new(Environment::new()))),
            config: Arc::new(Mutex::new(config)),
        }
    }

    pub fn new() -> EnvironmentSyncer {
        EnvironmentSyncer {
            env: Arc::new(Mutex::new(Box::new(Environment::new()))),
            config: Arc::new(Mutex::new(Box::new(NuConfig::new()))),
        }
    }

    #[cfg(test)]
    pub fn set_config(&mut self, config: Box<dyn Conf>) {
        self.config = Arc::new(Mutex::new(config));
    }

    pub fn get_config(&self) -> Box<dyn Conf> {
        let config = self.config.lock();

        config.clone_box()
    }

    pub fn load_environment(&mut self) {
        let config = self.config.lock();

        self.env = Arc::new(Mutex::new(Box::new(Environment::from_config(&*config))));
    }

    pub fn did_config_change(&mut self) -> bool {
        let config = self.config.lock();
        config.is_modified().unwrap_or(false)
    }

    pub fn reload(&mut self) {
        let mut config = self.config.lock();
        config.reload();

        let mut environment = self.env.lock();
        environment.morph(&*config);
    }

    pub fn autoenv(&self, ctx: &mut EvaluationContext) -> Result<(), ShellError> {
        let mut environment = self.env.lock();
        let recently_used = ctx
            .user_recently_used_autoenv_untrust
            .load(Ordering::SeqCst);
        let auto = environment.autoenv(recently_used);
        ctx.user_recently_used_autoenv_untrust
            .store(false, Ordering::SeqCst);
        auto
    }

    pub fn sync_env_vars(&mut self, ctx: &mut EvaluationContext) {
        let mut environment = self.env.lock();

        if environment.env().is_some() {
            for (name, value) in ctx.with_host(|host| host.vars()) {
                if name != "path" && name != "PATH" {
                    // account for new env vars present in the current session
                    // that aren't loaded from config.
                    environment.add_env(&name, &value);

                    // clear the env var from the session
                    // we are about to replace them
                    ctx.with_host(|host| host.env_rm(std::ffi::OsString::from(name)));
                }
            }

            if let Some(variables) = environment.env() {
                for var in variables.row_entries() {
                    if let Ok(string) = var.1.as_string() {
                        ctx.with_host(|host| {
                            host.env_set(
                                std::ffi::OsString::from(var.0),
                                std::ffi::OsString::from(string),
                            )
                        });
                    }
                }
            }
        }
    }

    pub fn sync_path_vars(&mut self, ctx: &mut EvaluationContext) {
        let mut environment = self.env.lock();

        if environment.path().is_some() {
            let native_paths = ctx.with_host(|host| host.env_get(std::ffi::OsString::from("PATH")));

            if let Some(native_paths) = native_paths {
                environment.add_path(native_paths);

                ctx.with_host(|host| {
                    host.env_rm(std::ffi::OsString::from("PATH"));
                });
            }

            if let Some(new_paths) = environment.path() {
                let prepared = std::env::join_paths(
                    new_paths
                        .table_entries()
                        .map(|p| p.as_string())
                        .filter_map(Result::ok),
                );

                if let Ok(paths_ready) = prepared {
                    ctx.with_host(|host| {
                        host.env_set(std::ffi::OsString::from("PATH"), paths_ready);
                    });
                }
            }
        }
    }

    #[cfg(test)]
    pub fn clear_env_vars(&mut self, ctx: &mut EvaluationContext) {
        for (key, _value) in ctx.with_host(|host| host.vars()) {
            if key != "path" && key != "PATH" {
                ctx.with_host(|host| host.env_rm(std::ffi::OsString::from(key)));
            }
        }
    }

    #[cfg(test)]
    pub fn clear_path_var(&mut self, ctx: &mut EvaluationContext) {
        ctx.with_host(|host| host.env_rm(std::ffi::OsString::from("PATH")));
    }
}

#[cfg(test)]
mod tests {
    use super::EnvironmentSyncer;
    use indexmap::IndexMap;
    use nu_data::config::tests::FakeConfig;
    use nu_engine::basic_evaluation_context;
    use nu_engine::Env;
    use nu_errors::ShellError;
    use nu_test_support::fs::Stub::FileWithContent;
    use nu_test_support::playground::Playground;
    use parking_lot::Mutex;
    use std::path::PathBuf;
    use std::sync::Arc;

    // This test fails on Linux.
    // It's possible it has something to do with the fake configuration
    // TODO: More tests.
    #[cfg(not(target_os = "linux"))]
    #[test]
    fn syncs_env_if_new_env_entry_is_added_to_an_existing_configuration() -> Result<(), ShellError>
    {
        let mut ctx = basic_evaluation_context()?;
        ctx.host = Arc::new(Mutex::new(Box::new(nu_engine::FakeHost::new())));

        let mut expected = IndexMap::new();
        expected.insert(
            "SHELL".to_string(),
            "/usr/bin/you_already_made_the_nu_choice".to_string(),
        );

        Playground::setup("syncs_env_from_config_updated_test_1", |dirs, sandbox| {
            sandbox.with_files(vec![
                FileWithContent(
                    "configuration.toml",
                    r#"
                    [env]
                    SHELL = "/usr/bin/you_already_made_the_nu_choice"
                "#,
                ),
                FileWithContent(
                    "updated_configuration.toml",
                    r#"
                    [env]
                    SHELL = "/usr/bin/you_already_made_the_nu_choice"
                    USER = "NUNO"
                "#,
                ),
            ]);

            let file = dirs.test().join("configuration.toml");
            let new_file = dirs.test().join("updated_configuration.toml");

            let fake_config = FakeConfig::new(&file);
            let mut actual = EnvironmentSyncer::with_config(Box::new(fake_config));

            // Here, the environment variables from the current session
            // are cleared since we will load and set them from the
            // configuration file
            actual.clear_env_vars(&mut ctx);

            // Nu loads the environment variables from the configuration file
            actual.load_environment();
            actual.sync_env_vars(&mut ctx);

            {
                let environment = actual.env.lock();
                let mut vars = IndexMap::new();
                environment
                    .env()
                    .expect("No variables in the environment.")
                    .row_entries()
                    .for_each(|(name, value)| {
                        vars.insert(
                            name.to_string(),
                            value.as_string().expect("Couldn't convert to string"),
                        );
                    });

                for k in expected.keys() {
                    assert!(vars.contains_key(k));
                }
            }

            assert!(!actual.did_config_change());

            // Replacing the newer configuration file to the existing one.
            let new_config_contents = std::fs::read_to_string(new_file).expect("Failed");
            std::fs::write(&file, &new_config_contents).expect("Failed");

            // A change has happened
            assert!(actual.did_config_change());

            // Syncer should reload and add new envs
            actual.reload();
            actual.sync_env_vars(&mut ctx);

            expected.insert("USER".to_string(), "NUNO".to_string());

            {
                let environment = actual.env.lock();
                let mut vars = IndexMap::new();
                environment
                    .env()
                    .expect("No variables in the environment.")
                    .row_entries()
                    .for_each(|(name, value)| {
                        vars.insert(
                            name.to_string(),
                            value.as_string().expect("Couldn't convert to string"),
                        );
                    });

                for k in expected.keys() {
                    assert!(vars.contains_key(k));
                }
            }
        });

        Ok(())
    }

    #[test]
    fn syncs_env_if_new_env_entry_in_session_is_not_in_configuration_file() -> Result<(), ShellError>
    {
        let mut ctx = basic_evaluation_context()?;
        ctx.host = Arc::new(Mutex::new(Box::new(nu_engine::FakeHost::new())));

        let mut expected = IndexMap::new();
        expected.insert(
            "SHELL".to_string(),
            "/usr/bin/you_already_made_the_nu_choice".to_string(),
        );
        expected.insert("USER".to_string(), "NUNO".to_string());

        Playground::setup("syncs_env_test_1", |dirs, sandbox| {
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
            let mut actual = EnvironmentSyncer::new();
            actual.set_config(Box::new(fake_config));

            // Here, the environment variables from the current session
            // are cleared since we will load and set them from the
            // configuration file (if any)
            actual.clear_env_vars(&mut ctx);

            // We explicitly simulate and add the USER variable to the current
            // session's environment variables with the value "NUNO".
            ctx.with_host(|test_host| {
                test_host.env_set(
                    std::ffi::OsString::from("USER"),
                    std::ffi::OsString::from("NUNO"),
                )
            });

            // Nu loads the environment variables from the configuration file (if any)
            actual.load_environment();

            // By this point, Nu has already loaded the environment variables
            // stored in the configuration file. Before continuing we check
            // if any new environment variables have been added from the ones loaded
            // in the configuration file.
            //
            // Nu sees the missing "USER" variable and accounts for it.
            actual.sync_env_vars(&mut ctx);

            // Confirms session environment variables are replaced from Nu configuration file
            // including the newer one accounted for.
            ctx.with_host(|test_host| {
                let var_user = test_host
                    .env_get(std::ffi::OsString::from("USER"))
                    .expect("Couldn't get USER var from host.")
                    .into_string()
                    .expect("Couldn't convert to string.");

                let var_shell = test_host
                    .env_get(std::ffi::OsString::from("SHELL"))
                    .expect("Couldn't get SHELL var from host.")
                    .into_string()
                    .expect("Couldn't convert to string.");

                let mut found = IndexMap::new();
                found.insert("SHELL".to_string(), var_shell);
                found.insert("USER".to_string(), var_user);

                for k in found.keys() {
                    assert!(expected.contains_key(k));
                }
            });

            // Now confirm in-memory environment variables synced appropriately
            // including the newer one accounted for.
            let environment = actual.env.lock();

            let mut vars = IndexMap::new();
            environment
                .env()
                .expect("No variables in the environment.")
                .row_entries()
                .for_each(|(name, value)| {
                    vars.insert(
                        name.to_string(),
                        value.as_string().expect("Couldn't convert to string"),
                    );
                });
            for k in expected.keys() {
                assert!(vars.contains_key(k));
            }
        });
        Ok(())
    }

    #[test]
    fn nu_envs_have_higher_priority_and_does_not_get_overwritten() -> Result<(), ShellError> {
        let mut ctx = basic_evaluation_context()?;
        ctx.host = Arc::new(Mutex::new(Box::new(nu_engine::FakeHost::new())));

        let mut expected = IndexMap::new();
        expected.insert(
            "SHELL".to_string(),
            "/usr/bin/you_already_made_the_nu_choice".to_string(),
        );

        Playground::setup("syncs_env_test_2", |dirs, sandbox| {
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
            let mut actual = EnvironmentSyncer::new();
            actual.set_config(Box::new(fake_config));

            actual.clear_env_vars(&mut ctx);

            ctx.with_host(|test_host| {
                test_host.env_set(
                    std::ffi::OsString::from("SHELL"),
                    std::ffi::OsString::from("/usr/bin/sh"),
                )
            });

            actual.load_environment();
            actual.sync_env_vars(&mut ctx);

            ctx.with_host(|test_host| {
                let var_shell = test_host
                    .env_get(std::ffi::OsString::from("SHELL"))
                    .expect("Couldn't get SHELL var from host.")
                    .into_string()
                    .expect("Couldn't convert to string.");

                let mut found = IndexMap::new();
                found.insert("SHELL".to_string(), var_shell);

                for k in found.keys() {
                    assert!(expected.contains_key(k));
                }
            });

            let environment = actual.env.lock();

            let mut vars = IndexMap::new();
            environment
                .env()
                .expect("No variables in the environment.")
                .row_entries()
                .for_each(|(name, value)| {
                    vars.insert(
                        name.to_string(),
                        value.as_string().expect("couldn't convert to string"),
                    );
                });
            for k in expected.keys() {
                assert!(vars.contains_key(k));
            }
        });

        Ok(())
    }

    #[test]
    fn syncs_path_if_new_path_entry_in_session_is_not_in_configuration_file(
    ) -> Result<(), ShellError> {
        let mut ctx = basic_evaluation_context()?;
        ctx.host = Arc::new(Mutex::new(Box::new(nu_engine::FakeHost::new())));

        let expected = std::env::join_paths(vec![
            PathBuf::from("/Users/andresrobalino/.volta/bin"),
            PathBuf::from("/Users/mosqueteros/bin"),
            PathBuf::from("/path/to/be/added"),
        ])
        .expect("Couldn't join paths.")
        .into_string()
        .expect("Couldn't convert to string.");

        Playground::setup("syncs_path_test_1", |dirs, sandbox| {
            sandbox.with_files(vec![FileWithContent(
                "configuration.toml",
                r#"
                    path = ["/Users/andresrobalino/.volta/bin", "/Users/mosqueteros/bin"]
                "#,
            )]);

            let mut file = dirs.test().clone();
            file.push("configuration.toml");

            let fake_config = FakeConfig::new(&file);
            let mut actual = EnvironmentSyncer::new();
            actual.set_config(Box::new(fake_config));

            // Here, the environment variables from the current session
            // are cleared since we will load and set them from the
            // configuration file (if any)
            actual.clear_path_var(&mut ctx);

            // We explicitly simulate and add the PATH variable to the current
            // session with the path "/path/to/be/added".
            ctx.with_host(|test_host| {
                test_host.env_set(
                    std::ffi::OsString::from("PATH"),
                    std::env::join_paths(vec![PathBuf::from("/path/to/be/added")])
                        .expect("Couldn't join paths."),
                )
            });

            // Nu loads the path variables from the configuration file (if any)
            actual.load_environment();

            // By this point, Nu has already loaded environment path variable
            // stored in the configuration file. Before continuing we check
            // if any new paths have been added from the ones loaded in the
            // configuration file.
            //
            // Nu sees the missing "/path/to/be/added" and accounts for it.
            actual.sync_path_vars(&mut ctx);

            ctx.with_host(|test_host| {
                let actual = test_host
                    .env_get(std::ffi::OsString::from("PATH"))
                    .expect("Couldn't get PATH var from host.")
                    .into_string()
                    .expect("Couldn't convert to string.");

                assert_eq!(actual, expected);
            });

            let environment = actual.env.lock();

            let paths = std::env::join_paths(
                &environment
                    .path()
                    .expect("No path variable in the environment.")
                    .table_entries()
                    .map(|value| value.as_string().expect("Couldn't convert to string"))
                    .map(PathBuf::from)
                    .collect::<Vec<_>>(),
            )
            .expect("Couldn't join paths.")
            .into_string()
            .expect("Couldn't convert to string.");

            assert_eq!(paths, expected);
        });

        Ok(())
    }

    #[test]
    fn nu_paths_have_higher_priority_and_new_paths_get_appended_to_the_end(
    ) -> Result<(), ShellError> {
        let mut ctx = basic_evaluation_context()?;
        ctx.host = Arc::new(Mutex::new(Box::new(nu_engine::FakeHost::new())));

        let expected = std::env::join_paths(vec![
            PathBuf::from("/Users/andresrobalino/.volta/bin"),
            PathBuf::from("/Users/mosqueteros/bin"),
            PathBuf::from("/path/to/be/added"),
        ])
        .expect("Couldn't join paths.")
        .into_string()
        .expect("Couldn't convert to string.");

        Playground::setup("syncs_path_test_2", |dirs, sandbox| {
            sandbox.with_files(vec![FileWithContent(
                "configuration.toml",
                r#"
                    path = ["/Users/andresrobalino/.volta/bin", "/Users/mosqueteros/bin"]
                "#,
            )]);

            let mut file = dirs.test().clone();
            file.push("configuration.toml");

            let fake_config = FakeConfig::new(&file);
            let mut actual = EnvironmentSyncer::new();
            actual.set_config(Box::new(fake_config));

            actual.clear_path_var(&mut ctx);

            ctx.with_host(|test_host| {
                test_host.env_set(
                    std::ffi::OsString::from("PATH"),
                    std::env::join_paths(vec![PathBuf::from("/path/to/be/added")])
                        .expect("Couldn't join paths."),
                )
            });

            actual.load_environment();
            actual.sync_path_vars(&mut ctx);

            ctx.with_host(|test_host| {
                let actual = test_host
                    .env_get(std::ffi::OsString::from("PATH"))
                    .expect("Couldn't get PATH var from host.")
                    .into_string()
                    .expect("Couldn't convert to string.");

                assert_eq!(actual, expected);
            });

            let environment = actual.env.lock();

            let paths = std::env::join_paths(
                &environment
                    .path()
                    .expect("No path variable in the environment.")
                    .table_entries()
                    .map(|value| value.as_string().expect("Couldn't convert to string"))
                    .map(PathBuf::from)
                    .collect::<Vec<_>>(),
            )
            .expect("Couldn't join paths.")
            .into_string()
            .expect("Couldn't convert to string.");

            assert_eq!(paths, expected);
        });

        Ok(())
    }
}
