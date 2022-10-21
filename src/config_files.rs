use log::info;
use nu_cli::{eval_config_contents, eval_source, report_error};
use nu_parser::ParseError;
use nu_path::canonicalize_with;
use nu_protocol::engine::{EngineState, Stack, StateWorkingSet};
use nu_protocol::{PipelineData, Span, Spanned};
use nu_utils::{get_default_config, get_default_env};
use std::fs::File;
use std::io::Write;

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

        match canonicalize_with(&file.item, cwd) {
            Ok(path) => {
                eval_config_contents(path, engine_state, stack);
            }
            Err(_) => {
                let e = ParseError::FileNotFound(file.item, file.span);
                report_error(&working_set, &e);
            }
        }
    } else if let Some(mut config_path) = nu_path::config_dir() {
        config_path.push(NUSHELL_FOLDER);

        // Create config directory if it does not exist
        if !config_path.exists() {
            if let Err(err) = std::fs::create_dir_all(&config_path) {
                eprintln!("Failed to create config directory: {}", err);
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
                "y" | "" => match File::create(&config_path) {
                    Ok(mut output) => match write!(output, "{}", config_file) {
                        Ok(_) => {
                            println!("Config file created at: {}", config_path.to_string_lossy())
                        }
                        Err(_) => {
                            eprintln!(
                                "Unable to write to {}, sourcing default file instead",
                                config_path.to_string_lossy(),
                            );
                            eval_default_config(engine_state, stack, config_file, is_env_config);
                            return;
                        }
                    },
                    Err(_) => {
                        eprintln!(
                            "Unable to create {}, sourcing default file instead",
                            config_file
                        );
                        eval_default_config(engine_state, stack, config_file, is_env_config);
                        return;
                    }
                },
                _ => {
                    eval_default_config(engine_state, stack, config_file, is_env_config);
                    return;
                }
            }
        }

        eval_config_contents(config_path, engine_state, stack);
    }

    info!("read_config_file {}:{}:{}", file!(), line!(), column!());
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
        PipelineData::new(Span::new(0, 0)),
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
        PipelineData::new(Span::new(0, 0)),
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
