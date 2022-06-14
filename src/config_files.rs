use log::info;
use nu_cli::{eval_config_contents, eval_source, report_error};
use nu_parser::ParseError;
use nu_path::canonicalize_with;
use nu_protocol::engine::{EngineState, Stack, StateWorkingSet};
use nu_protocol::{PipelineData, Span, Spanned};
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
    is_perf_true: bool,
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
                include_str!("../docs/sample_config/default_env.nu")
            } else {
                include_str!("../docs/sample_config/default_config.nu")
            };

            match answer.to_lowercase().trim() {
                "y" | "" => {
                    let mut output = File::create(&config_path).expect("Unable to create file");
                    write!(output, "{}", config_file).expect("Unable to write to config file");
                    println!("Config file created at: {}", config_path.to_string_lossy());
                }
                _ => {
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
                    return;
                }
            }
        }

        eval_config_contents(config_path, engine_state, stack);
    }

    if is_perf_true {
        info!("read_config_file {}:{}:{}", file!(), line!(), column!());
    }
}
pub(crate) fn read_loginshell_file(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    is_perf_true: bool,
) {
    // read and execute loginshell file if exists
    if let Some(mut config_path) = nu_path::config_dir() {
        config_path.push(NUSHELL_FOLDER);
        config_path.push(LOGINSHELL_FILE);

        if config_path.exists() {
            eval_config_contents(config_path, engine_state, stack);
        }
    }

    if is_perf_true {
        info!("read_loginshell_file {}:{}:{}", file!(), line!(), column!());
    }
}
