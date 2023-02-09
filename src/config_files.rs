use log::info;
#[cfg(feature = "plugin")]
use nu_cli::read_plugin_file;
use nu_cli::{eval_config_contents, eval_source, report_error};
use nu_parser::ParseError;
use nu_path::canonicalize_with;
use nu_protocol::engine::{EngineState, Stack, StateWorkingSet};
use nu_protocol::{PipelineData, Spanned};
use nu_utils::{get_default_config, get_default_env};
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub(crate) const NUSHELL_FOLDER: &str = "nushell";
const CONFIG_FILE: &str = "config.nu";
const ENV_FILE: &str = "env.nu";
const LOGINSHELL_FILE: &str = "login.nu";

pub(crate) fn read_config_file(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    config_file: Option<Spanned<String>>,
    is_env_config: bool,
) {
    // Load config startup file
    if let Some(file) = config_file {
        let working_set = StateWorkingSet::new(engine_state);
        let cwd = working_set.get_cwd();

        if let Ok(path) = canonicalize_with(&file.item, cwd) {
            eval_config_contents(path, engine_state, stack);
        } else {
            let e = ParseError::FileNotFound(file.item, file.span);
            report_error(&working_set, &e);
        }
    } else if let Some(mut config_path) = nu_path::config_dir() {
        config_path.push(NUSHELL_FOLDER);

        // Create config directory if it does not exist
        if !config_path.exists() {
            if let Err(err) = std::fs::create_dir_all(&config_path) {
                eprintln!("Failed to create config directory: {err}");
                return;
            }
        }

        config_path.push(if is_env_config { ENV_FILE } else { CONFIG_FILE });

        if !config_path.exists() {
            let file_msg = if is_env_config {
                "environment config"
            } else {
                "config"
            };
            println!(
                "No {} file found at {}",
                file_msg,
                config_path.to_string_lossy()
            );
            println!("Would you like to create one with defaults (Y/n): ");

            let mut answer = String::new();
            std::io::stdin()
                .read_line(&mut answer)
                .expect("Failed to read user input");

            let config_file = if is_env_config {
                get_default_env()
            } else {
                get_default_config()
            };

            match answer.to_lowercase().trim() {
                "y" | "" => {
                    if let Ok(mut output) = File::create(&config_path) {
                        if write!(output, "{config_file}").is_ok() {
                            let config_type = if is_env_config {
                                "Environment config"
                            } else {
                                "Config"
                            };
                            println!(
                                "{} file created at: {}",
                                config_type,
                                config_path.to_string_lossy()
                            );
                        } else {
                            eprintln!(
                                "Unable to write to {}, sourcing default file instead",
                                config_path.to_string_lossy(),
                            );
                            eval_default_config(engine_state, stack, config_file, is_env_config);
                            return;
                        }
                    } else {
                        eprintln!("Unable to create {config_file}, sourcing default file instead");
                        eval_default_config(engine_state, stack, config_file, is_env_config);
                        return;
                    }
                }
                _ => {
                    eval_default_config(engine_state, stack, config_file, is_env_config);
                    return;
                }
            }
        }

        eval_config_contents(config_path, engine_state, stack);
    }
}

pub(crate) fn read_loginshell_file(engine_state: &mut EngineState, stack: &mut Stack) {
    // read and execute loginshell file if exists
    if let Some(mut config_path) = nu_path::config_dir() {
        config_path.push(NUSHELL_FOLDER);
        config_path.push(LOGINSHELL_FILE);

        if config_path.exists() {
            eval_config_contents(config_path, engine_state, stack);
        }
    }

    info!("read_loginshell_file {}:{}:{}", file!(), line!(), column!());
}

pub(crate) fn read_default_env_file(engine_state: &mut EngineState, stack: &mut Stack) {
    let config_file = get_default_env();
    eval_source(
        engine_state,
        stack,
        config_file.as_bytes(),
        "default_env.nu",
        PipelineData::empty(),
        false,
    );

    info!("read_config_file {}:{}:{}", file!(), line!(), column!());
    // Merge the environment in case env vars changed in the config
    match nu_engine::env::current_dir(engine_state, stack) {
        Ok(cwd) => {
            if let Err(e) = engine_state.merge_env(stack, cwd) {
                let working_set = StateWorkingSet::new(engine_state);
                report_error(&working_set, &e);
            }
        }
        Err(e) => {
            let working_set = StateWorkingSet::new(engine_state);
            report_error(&working_set, &e);
        }
    }
}

fn eval_default_config(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    config_file: &str,
    is_env_config: bool,
) {
    println!("Continuing without config file");
    // Just use the contents of "default_config.nu" or "default_env.nu"
    eval_source(
        engine_state,
        stack,
        config_file.as_bytes(),
        if is_env_config {
            "default_env.nu"
        } else {
            "default_config.nu"
        },
        PipelineData::empty(),
        false,
    );

    // Merge the environment in case env vars changed in the config
    match nu_engine::env::current_dir(engine_state, stack) {
        Ok(cwd) => {
            if let Err(e) = engine_state.merge_env(stack, cwd) {
                let working_set = StateWorkingSet::new(engine_state);
                report_error(&working_set, &e);
            }
        }
        Err(e) => {
            let working_set = StateWorkingSet::new(engine_state);
            report_error(&working_set, &e);
        }
    }
}

pub(crate) fn setup_config(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    #[cfg(feature = "plugin")] plugin_file: Option<Spanned<String>>,
    config_file: Option<Spanned<String>>,
    env_file: Option<Spanned<String>>,
    is_login_shell: bool,
) {
    #[cfg(feature = "plugin")]
    read_plugin_file(engine_state, stack, plugin_file, NUSHELL_FOLDER);

    read_config_file(engine_state, stack, env_file, true);
    read_config_file(engine_state, stack, config_file, false);

    if is_login_shell {
        read_loginshell_file(engine_state, stack);
    }

    // Give a warning if we see `$config` for a few releases
    {
        let working_set = StateWorkingSet::new(engine_state);
        if working_set.find_variable(b"$config").is_some() {
            println!("warning: use `let-env config = ...` instead of `let config = ...`");
        }
    }
}

pub(crate) fn set_config_path(
    engine_state: &mut EngineState,
    cwd: &Path,
    default_config_name: &str,
    key: &str,
    config_file: &Option<Spanned<String>>,
) {
    let config_path = match config_file {
        Some(s) => canonicalize_with(&s.item, cwd).ok(),
        None => nu_path::config_dir().map(|mut p| {
            p.push(NUSHELL_FOLDER);
            p.push(default_config_name);
            p
        }),
    };

    if let Some(path) = config_path {
        engine_state.set_config_path(key, path);
    }
}
