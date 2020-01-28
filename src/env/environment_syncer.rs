use crate::context::Context;
use crate::data::config::{Conf, NuConfig};
use crate::env::environment::{Env, Environment};
use parking_lot::Mutex;
use std::sync::Arc;

pub struct EnvironmentSyncer {
    pub env: Arc<Mutex<Box<Environment>>>,
    pub config: Arc<Box<dyn Conf>>,
}

impl EnvironmentSyncer {
    pub fn new() -> EnvironmentSyncer {
        EnvironmentSyncer {
            env: Arc::new(Mutex::new(Box::new(Environment::new()))),
            config: Arc::new(Box::new(NuConfig::new())),
        }
    }

    #[cfg(test)]
    pub fn set_config(&mut self, config: Box<dyn Conf>) {
        self.config = Arc::new(config);
    }

    pub fn load_environment(&mut self) {
        let config = self.config.clone();

        self.env = Arc::new(Mutex::new(Box::new(Environment::from_config(&*config))));
    }

    pub fn reload(&mut self) {
        self.config.reload();

        let mut environment = self.env.lock();
        environment.morph(&*self.config);
    }

    pub fn sync_env_vars(&mut self, ctx: &mut Context) {
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
                for var in nu_value_ext::row_entries(&variables) {
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

    pub fn sync_path_vars(&mut self, ctx: &mut Context) {
        let mut environment = self.env.lock();

        if environment.path().is_some() {
            let native_paths = ctx.with_host(|host| host.env_get(std::ffi::OsString::from("PATH")));

            if let Some(native_paths) = native_paths {
                environment.add_path(native_paths);
            }

            ctx.with_host(|host| {
                host.env_rm(std::ffi::OsString::from("PATH"));
            });

            if let Some(new_paths) = environment.path() {
                let prepared = std::env::join_paths(
                    nu_value_ext::table_entries(&new_paths)
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
    pub fn clear_env_vars(&mut self, ctx: &mut Context) {
        for (key, _value) in ctx.with_host(|host| host.vars()) {
            if key != "path" && key != "PATH" {
                ctx.with_host(|host| host.env_rm(std::ffi::OsString::from(key)));
            }
        }
    }

    #[cfg(test)]
    pub fn clear_path_var(&mut self, ctx: &mut Context) {
        ctx.with_host(|host| host.env_rm(std::ffi::OsString::from("PATH")));
    }
}

#[cfg(test)]
mod tests {
    use super::EnvironmentSyncer;
    use crate::context::Context;
    use crate::data::config::tests::FakeConfig;
    use crate::env::environment::Env;
    use nu_errors::ShellError;
    use nu_test_support::fs::Stub::FileWithContent;
    use nu_test_support::playground::Playground;
    use parking_lot::Mutex;
    use std::path::PathBuf;
    use std::sync::Arc;

    #[test]
    fn syncs_env_if_new_env_entry_in_session_is_not_in_configuration_file() -> Result<(), ShellError>
    {
        let mut ctx = Context::basic()?;
        ctx.host = Arc::new(Mutex::new(Box::new(crate::env::host::FakeHost::new())));

        let expected = vec![
            (
                "SHELL".to_string(),
                "/usr/bin/you_already_made_the_nu_choice".to_string(),
            ),
            ("USER".to_string(), "NUNO".to_string()),
        ];

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

                let actual = vec![
                    ("SHELL".to_string(), var_shell),
                    ("USER".to_string(), var_user),
                ];

                assert_eq!(actual, expected);
            });

            // Now confirm in-memory environment variables synced appropiately
            // including the newer one accounted for.
            let environment = actual.env.lock();

            let vars = nu_value_ext::row_entries(
                &environment.env().expect("No variables in the environment."),
            )
            .map(|(name, value)| {
                (
                    name.to_string(),
                    value.as_string().expect("Couldn't convert to string"),
                )
            })
            .collect::<Vec<_>>();

            assert_eq!(vars, expected);
        });

        Ok(())
    }

    #[test]
    fn nu_envs_have_higher_priority_and_does_not_get_overwritten() -> Result<(), ShellError> {
        let mut ctx = Context::basic()?;
        ctx.host = Arc::new(Mutex::new(Box::new(crate::env::host::FakeHost::new())));

        let expected = vec![(
            "SHELL".to_string(),
            "/usr/bin/you_already_made_the_nu_choice".to_string(),
        )];

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

                let actual = vec![("SHELL".to_string(), var_shell)];

                assert_eq!(actual, expected);
            });

            let environment = actual.env.lock();

            let vars = nu_value_ext::row_entries(
                &environment.env().expect("No variables in the environment."),
            )
            .map(|(name, value)| {
                (
                    name.to_string(),
                    value.as_string().expect("Couldn't convert to string"),
                )
            })
            .collect::<Vec<_>>();

            assert_eq!(vars, expected);
        });

        Ok(())
    }

    #[test]
    fn syncs_path_if_new_path_entry_in_session_is_not_in_configuration_file(
    ) -> Result<(), ShellError> {
        let mut ctx = Context::basic()?;
        ctx.host = Arc::new(Mutex::new(Box::new(crate::env::host::FakeHost::new())));

        let expected = std::env::join_paths(vec![
            PathBuf::from("/path/to/be/added"),
            PathBuf::from("/Users/andresrobalino/.volta/bin"),
            PathBuf::from("/Users/mosqueteros/bin"),
        ])
        .expect("Couldn't join paths.")
        .into_string()
        .expect("Couldn't convert to string.");

        Playground::setup("syncs_path_test_3", |dirs, sandbox| {
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
                &nu_value_ext::table_entries(
                    &environment
                        .path()
                        .expect("No path variable in the environment."),
                )
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
