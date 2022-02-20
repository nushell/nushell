use crate::is_perf_true;
use crate::utils::{eval_source, report_error};
use log::info;
use nu_parser::ParseError;
use nu_path::canonicalize_with;
use nu_protocol::engine::{EngineState, Stack, StateDelta, StateWorkingSet};
use nu_protocol::{PipelineData, Span, Spanned};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

const NUSHELL_FOLDER: &str = "nushell";
const CONFIG_FILE: &str = "config.nu";
const HISTORY_FILE: &str = "history.txt";
#[cfg(feature = "plugin")]
const PLUGIN_FILE: &str = "plugin.nu";

#[cfg(feature = "plugin")]
pub(crate) fn read_plugin_file(engine_state: &mut EngineState, stack: &mut Stack) {
    // Reading signatures from signature file
    // The plugin.nu file stores the parsed signature collected from each registered plugin
    if let Some(mut plugin_path) = nu_path::config_dir() {
        // Path to store plugins signatures
        plugin_path.push(NUSHELL_FOLDER);
        plugin_path.push(PLUGIN_FILE);
        engine_state.plugin_signatures = Some(plugin_path.clone());

        let plugin_filename = plugin_path.to_string_lossy().to_owned();

        if let Ok(contents) = std::fs::read(&plugin_path) {
            eval_source(
                engine_state,
                stack,
                &contents,
                &plugin_filename,
                PipelineData::new(Span::new(0, 0)),
            );
        }
    }
    if is_perf_true() {
        info!("read_plugin_file {}:{}:{}", file!(), line!(), column!());
    }
}

pub(crate) fn read_config_file(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    config_file: Option<Spanned<String>>,
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

        config_path.push(CONFIG_FILE);

        if !config_path.exists() {
            println!("No config file found at {:?}", config_path);
            println!("Would you like to create one (Y/n): ");

            let mut answer = String::new();
            std::io::stdin()
                .read_line(&mut answer)
                .expect("Failed to read user input");

            match answer.to_lowercase().trim() {
                "y" => {
                    let mut output = File::create(&config_path).expect("Unable to create file");
                    let config_file = include_str!("default_config.nu");
                    write!(output, "{}", config_file).expect("Unable to write to config file");
                    println!("Config file created {:?}", config_path);
                }
                _ => {
                    println!("Continuing without config file");
                    return;
                }
            }
        }

        eval_config_contents(config_path, engine_state, stack);
    }

    if is_perf_true() {
        info!("read_config_file {}:{}:{}", file!(), line!(), column!());
    }
}

fn eval_config_contents(config_path: PathBuf, engine_state: &mut EngineState, stack: &mut Stack) {
    if config_path.exists() & config_path.is_file() {
        let config_filename = config_path.to_string_lossy().to_owned();

        if let Ok(contents) = std::fs::read(&config_path) {
            eval_source(
                engine_state,
                stack,
                &contents,
                &config_filename,
                PipelineData::new(Span::new(0, 0)),
            );

            // Merge the delta in case env vars changed in the config
            match nu_engine::env::current_dir(engine_state, stack) {
                Ok(cwd) => {
                    if let Err(e) = engine_state.merge_delta(StateDelta::new(), Some(stack), cwd) {
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
    }
}

pub(crate) fn create_history_path() -> Option<PathBuf> {
    nu_path::config_dir().and_then(|mut history_path| {
        history_path.push(NUSHELL_FOLDER);
        history_path.push(HISTORY_FILE);

        if !history_path.exists() {
            // Creating an empty file to store the history
            match std::fs::File::create(&history_path) {
                Ok(_) => Some(history_path),
                Err(_) => None,
            }
        } else {
            Some(history_path)
        }
    })
}

