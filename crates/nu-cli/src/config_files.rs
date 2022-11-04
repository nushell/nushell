use crate::util::{eval_source, report_error};
#[cfg(feature = "plugin")]
use log::info;
#[cfg(feature = "plugin")]
use nu_parser::ParseError;
#[cfg(feature = "plugin")]
use nu_path::canonicalize_with;
use nu_protocol::engine::{EngineState, Stack, StateWorkingSet};
#[cfg(feature = "plugin")]
use nu_protocol::Spanned;
use nu_protocol::{HistoryFileFormat, PipelineData, Span};
use std::path::PathBuf;

#[cfg(feature = "plugin")]
const PLUGIN_FILE: &str = "plugin.nu";

const HISTORY_FILE_TXT: &str = "history.txt";
const HISTORY_FILE_SQLITE: &str = "history.sqlite3";

#[cfg(feature = "plugin")]
pub fn read_plugin_file(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    plugin_file: Option<Spanned<String>>,
    storage_path: &str,
) {
    // Reading signatures from signature file
    // The plugin.nu file stores the parsed signature collected from each registered plugin
    add_plugin_file(engine_state, plugin_file, storage_path);

    let plugin_path = engine_state.plugin_signatures.clone();
    if let Some(plugin_path) = plugin_path {
        let plugin_filename = plugin_path.to_string_lossy();

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

    info!("read_plugin_file {}:{}:{}", file!(), line!(), column!());
}

#[cfg(feature = "plugin")]
pub fn add_plugin_file(
    engine_state: &mut EngineState,
    plugin_file: Option<Spanned<String>>,
    storage_path: &str,
) {
    if let Some(plugin_file) = plugin_file {
        let working_set = StateWorkingSet::new(engine_state);
        let cwd = working_set.get_cwd();

        match canonicalize_with(&plugin_file.item, cwd) {
            Ok(path) => engine_state.plugin_signatures = Some(path),
            Err(_) => {
                let e = ParseError::FileNotFound(plugin_file.item, plugin_file.span);
                report_error(&working_set, &e);
            }
        }
    } else if let Some(mut plugin_path) = nu_path::config_dir() {
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
        let config_filename = config_path.to_string_lossy();

        if let Ok(contents) = std::fs::read(&config_path) {
            eval_source(
                engine_state,
                stack,
                &contents,
                &config_filename,
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
    }
}

pub(crate) fn get_history_path(storage_path: &str, mode: HistoryFileFormat) -> Option<PathBuf> {
    nu_path::config_dir().map(|mut history_path| {
        history_path.push(storage_path);
        history_path.push(match mode {
            HistoryFileFormat::PlainText => HISTORY_FILE_TXT,
            HistoryFileFormat::Sqlite => HISTORY_FILE_SQLITE,
        });
        history_path
    })
}
