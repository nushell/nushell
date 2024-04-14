use crate::util::eval_source;
#[cfg(feature = "plugin")]
use nu_path::canonicalize_with;
use nu_protocol::{
    engine::{EngineState, Stack, StateWorkingSet},
    report_error, HistoryFileFormat, PipelineData,
};
#[cfg(feature = "plugin")]
use nu_protocol::{ParseError, Spanned};
#[cfg(feature = "plugin")]
use nu_utils::utils::perf;
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
    let mut start_time = std::time::Instant::now();
    // Reading signatures from signature file
    // The plugin.nu file stores the parsed signature collected from each registered plugin
    add_plugin_file(engine_state, plugin_file, storage_path);
    perf(
        "add plugin file to engine_state",
        start_time,
        file!(),
        line!(),
        column!(),
        engine_state.get_config().use_ansi_coloring,
    );

    start_time = std::time::Instant::now();
    let plugin_path = engine_state.plugin_signatures.clone();
    if let Some(plugin_path) = plugin_path {
        let plugin_filename = plugin_path.to_string_lossy();
        let plug_path = plugin_filename.to_string();

        if let Ok(contents) = std::fs::read(&plugin_path) {
            perf(
                &format!("read plugin file {}", &plug_path),
                start_time,
                file!(),
                line!(),
                column!(),
                engine_state.get_config().use_ansi_coloring,
            );
            start_time = std::time::Instant::now();
            eval_source(
                engine_state,
                stack,
                &contents,
                &plugin_filename,
                PipelineData::empty(),
                false,
            );
            perf(
                &format!("eval_source plugin file {}", &plug_path),
                start_time,
                file!(),
                line!(),
                column!(),
                engine_state.get_config().use_ansi_coloring,
            );
        }
    }
}

#[cfg(feature = "plugin")]
pub fn add_plugin_file(
    engine_state: &mut EngineState,
    plugin_file: Option<Spanned<String>>,
    storage_path: &str,
) {
    let working_set = StateWorkingSet::new(engine_state);
    let cwd = working_set.get_cwd();

    if let Some(plugin_file) = plugin_file {
        if let Ok(path) = canonicalize_with(&plugin_file.item, cwd) {
            engine_state.plugin_signatures = Some(path)
        } else {
            let e = ParseError::FileNotFound(plugin_file.item, plugin_file.span);
            report_error(&working_set, &e);
        }
    } else if let Some(mut plugin_path) = nu_path::config_dir() {
        // Path to store plugins signatures
        plugin_path.push(storage_path);
        let mut plugin_path = canonicalize_with(&plugin_path, &cwd).unwrap_or(plugin_path);
        plugin_path.push(PLUGIN_FILE);
        let plugin_path = canonicalize_with(&plugin_path, &cwd).unwrap_or(plugin_path);
        engine_state.plugin_signatures = Some(plugin_path);
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
            // Set the current active file to the config file.
            let prev_file = engine_state.file.take();
            engine_state.file = Some(config_path.clone());

            eval_source(
                engine_state,
                stack,
                &contents,
                &config_filename,
                PipelineData::empty(),
                false,
            );

            // Restore the current active file.
            engine_state.file = prev_file;

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
