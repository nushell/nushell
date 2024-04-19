use crate::util::eval_source;
#[cfg(feature = "plugin")]
use nu_path::canonicalize_with;
use nu_protocol::{
    engine::{EngineState, Stack, StateWorkingSet},
    report_error, HistoryFileFormat, PipelineData,
};
#[cfg(feature = "plugin")]
use nu_protocol::{ParseError, PluginCacheFile, Spanned};
#[cfg(feature = "plugin")]
use nu_utils::utils::perf;
use std::path::PathBuf;

#[cfg(feature = "plugin")]
const PLUGIN_FILE: &str = "plugin.msgpackz";

const HISTORY_FILE_TXT: &str = "history.txt";
const HISTORY_FILE_SQLITE: &str = "history.sqlite3";

#[cfg(feature = "plugin")]
pub fn read_plugin_file(
    engine_state: &mut EngineState,
    plugin_file: Option<Spanned<String>>,
    storage_path: &str,
) {
    use nu_protocol::{report_error_new, ShellError};

    let span = plugin_file.as_ref().map(|s| s.span);

    let mut start_time = std::time::Instant::now();
    // Reading signatures from signature file
    // The plugin.msgpackz file stores the parsed signature collected from each registered plugin
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
    let plugin_path = engine_state.plugin_path.clone();
    if let Some(plugin_path) = plugin_path {
        // Open the plugin file
        let mut file = match std::fs::File::open(&plugin_path) {
            Ok(file) => file,
            Err(err) => {
                if err.kind() == std::io::ErrorKind::NotFound {
                    log::warn!("Plugin file not found: {}", plugin_path.display());
                } else {
                    report_error_new(
                        engine_state,
                        &ShellError::GenericError {
                            error: format!(
                                "Error while opening plugin cache file: {}",
                                plugin_path.display()
                            ),
                            msg: "plugin path defined here".into(),
                            span,
                            help: None,
                            inner: vec![err.into()],
                        },
                    );
                }
                return;
            }
        };

        // Abort if the file is empty.
        if file.metadata().is_ok_and(|m| m.len() == 0) {
            log::warn!(
                "Not reading plugin file because it's empty: {}",
                plugin_path.display()
            );
            return;
        }

        // Read the contents of the plugin file
        let contents = match PluginCacheFile::read_from(&mut file, span) {
            Ok(contents) => contents,
            Err(err) => {
                log::warn!("Failed to read plugin cache file: {err:?}");
                report_error_new(
                    engine_state,
                    &ShellError::GenericError {
                        error: format!(
                            "Error while reading plugin cache file: {}",
                            plugin_path.display()
                        ),
                        msg: "plugin path defined here".into(),
                        span,
                        help: Some(
                            "you might try deleting the file and registering all of your \
                                plugins again"
                                .into(),
                        ),
                        inner: vec![],
                    },
                );
                return;
            }
        };

        perf(
            &format!("read plugin file {}", plugin_path.display()),
            start_time,
            file!(),
            line!(),
            column!(),
            engine_state.get_config().use_ansi_coloring,
        );
        start_time = std::time::Instant::now();

        let mut working_set = StateWorkingSet::new(engine_state);

        nu_plugin::load_plugin_file(&mut working_set, &contents, span);

        if let Err(err) = engine_state.merge_delta(working_set.render()) {
            report_error_new(engine_state, &err);
            return;
        }

        perf(
            &format!("load plugin file {}", plugin_path.display()),
            start_time,
            file!(),
            line!(),
            column!(),
            engine_state.get_config().use_ansi_coloring,
        );
    }
}

#[cfg(feature = "plugin")]
pub fn add_plugin_file(
    engine_state: &mut EngineState,
    plugin_file: Option<Spanned<String>>,
    storage_path: &str,
) {
    use std::path::Path;

    let working_set = StateWorkingSet::new(engine_state);
    let cwd = working_set.get_cwd();

    if let Some(plugin_file) = plugin_file {
        let path = Path::new(&plugin_file.item);
        let path_dir = path.parent().unwrap_or(path);
        // Just try to canonicalize the directory of the plugin file first.
        if let Ok(path_dir) = canonicalize_with(path_dir, &cwd) {
            // Try to canonicalize the actual filename, but it's ok if that fails. The file doesn't
            // have to exist.
            let path = path_dir.join(path.file_name().unwrap_or(path.as_os_str()));
            let path = canonicalize_with(&path, &cwd).unwrap_or(path);
            engine_state.plugin_path = Some(path)
        } else {
            // It's an error if the directory for the plugin file doesn't exist.
            report_error(
                &working_set,
                &ParseError::FileNotFound(
                    path_dir.to_string_lossy().into_owned(),
                    plugin_file.span,
                ),
            );
        }
    } else if let Some(mut plugin_path) = nu_path::config_dir() {
        // Path to store plugins signatures
        plugin_path.push(storage_path);
        let mut plugin_path = canonicalize_with(&plugin_path, &cwd).unwrap_or(plugin_path);
        plugin_path.push(PLUGIN_FILE);
        let plugin_path = canonicalize_with(&plugin_path, &cwd).unwrap_or(plugin_path);
        engine_state.plugin_path = Some(plugin_path);
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
