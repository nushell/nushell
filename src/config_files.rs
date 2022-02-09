use crate::is_perf_true;
use crate::utils::{eval_source, report_error};
use log::info;
use nu_protocol::engine::{EngineState, Stack, StateDelta, StateWorkingSet};
use std::path::PathBuf;

const NUSHELL_FOLDER: &str = "nushell";
const PLUGIN_FILE: &str = "plugin.nu";
const CONFIG_FILE: &str = "config.nu";
const HISTORY_FILE: &str = "history.txt";

pub(crate) fn read_plugin_file(engine_state: &mut EngineState, stack: &mut Stack) {
    // Reading signatures from signature file
    // The plugin.nu file stores the parsed signature collected from each registered plugin
    if let Some(mut plugin_path) = nu_path::config_dir() {
        // Path to store plugins signatures
        plugin_path.push(NUSHELL_FOLDER);
        plugin_path.push(PLUGIN_FILE);
        engine_state.plugin_signatures = Some(plugin_path.clone());

        let plugin_filename = plugin_path.to_string_lossy().to_owned();

        if let Ok(contents) = std::fs::read_to_string(&plugin_path) {
            eval_source(engine_state, stack, &contents, &plugin_filename);
        }
    }
    if is_perf_true() {
        info!("read_plugin_file {}:{}:{}", file!(), line!(), column!());
    }
}

pub(crate) fn read_config_file(engine_state: &mut EngineState, stack: &mut Stack) {
    // Load config startup file
    if let Some(mut config_path) = nu_path::config_dir() {
        config_path.push(NUSHELL_FOLDER);

        // Create config directory if it does not exist
        if !config_path.exists() {
            if let Err(err) = std::fs::create_dir_all(&config_path) {
                eprintln!("Failed to create config directory: {}", err);
            }
        } else {
            config_path.push(CONFIG_FILE);

            if config_path.exists() {
                // FIXME: remove this message when we're ready
                //println!("Loading config from: {:?}", config_path);
                let config_filename = config_path.to_string_lossy().to_owned();

                if let Ok(contents) = std::fs::read_to_string(&config_path) {
                    eval_source(engine_state, stack, &contents, &config_filename);
                    // Merge the delta in case env vars changed in the config
                    match nu_engine::env::current_dir(engine_state, stack) {
                        Ok(cwd) => {
                            if let Err(e) =
                                engine_state.merge_delta(StateDelta::new(), Some(stack), cwd)
                            {
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
    }
    if is_perf_true() {
        info!("read_config_file {}:{}:{}", file!(), line!(), column!());
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
