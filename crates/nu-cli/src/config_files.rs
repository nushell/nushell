use crate::util::{eval_source, report_error};
#[cfg(feature = "plugin")]
use log::info;
use nu_protocol::engine::{EngineState, Stack, StateDelta, StateWorkingSet};
use nu_protocol::{PipelineData, Span};
use std::path::PathBuf;

#[cfg(feature = "plugin")]
const PLUGIN_FILE: &str = "plugin.nu";

#[cfg(feature = "plugin")]
pub fn read_plugin_file(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    storage_path: &str,
    is_perf_true: bool,
) {
    // Reading signatures from signature file
    // The plugin.nu file stores the parsed signature collected from each registered plugin
    add_plugin_file(engine_state, storage_path);

    let plugin_path = engine_state.plugin_signatures.clone();
    if let Some(plugin_path) = plugin_path {
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

    if is_perf_true {
        info!("read_plugin_file {}:{}:{}", file!(), line!(), column!());
    }
}

#[cfg(feature = "plugin")]
pub fn add_plugin_file(engine_state: &mut EngineState, storage_path: &str) {
    if let Some(mut plugin_path) = nu_path::config_dir() {
        // Path to store plugins signatures
        plugin_path.push(storage_path);
        plugin_path.push(PLUGIN_FILE);
        engine_state.plugin_signatures = Some(plugin_path.clone());
    }
}

pub fn eval_config_contents(
    config_path: PathBuf,
    engine_state: &mut EngineState,
    stack: &mut Stack,
) {
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
                    if let Err(e) =
                        engine_state.merge_delta(StateDelta::new(engine_state), Some(stack), cwd)
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
