use crate::util::eval_source;
#[cfg(feature = "plugin")]
use nu_path::canonicalize_with;
#[cfg(feature = "plugin")]
use nu_protocol::{engine::StateWorkingSet, report_error, ParseError, PluginRegistryFile, Spanned};
use nu_protocol::{
    engine::{EngineState, Stack},
    report_error_new, HistoryFileFormat, PipelineData,
};
#[cfg(feature = "plugin")]
use nu_utils::utils::perf;
use std::path::PathBuf;

#[cfg(feature = "plugin")]
const PLUGIN_FILE: &str = "plugin.msgpackz";
#[cfg(feature = "plugin")]
const OLD_PLUGIN_FILE: &str = "plugin.nu";

const HISTORY_FILE_TXT: &str = "history.txt";
const HISTORY_FILE_SQLITE: &str = "history.sqlite3";

#[cfg(feature = "plugin")]
pub fn read_plugin_file(
    engine_state: &mut EngineState,
    plugin_file: Option<Spanned<String>>,
    storage_path: &str,
) {
    use nu_protocol::ShellError;
    use std::path::Path;

    let span = plugin_file.as_ref().map(|s| s.span);

    // Check and warn + abort if this is a .nu plugin file
    if plugin_file
        .as_ref()
        .and_then(|p| Path::new(&p.item).extension())
        .is_some_and(|ext| ext == "nu")
    {
        report_error_new(
            engine_state,
            &ShellError::GenericError {
                error: "Wrong plugin file format".into(),
                msg: ".nu plugin files are no longer supported".into(),
                span,
                help: Some("please recreate this file in the new .msgpackz format".into()),
                inner: vec![],
            },
        );
        return;
    }

    let mut start_time = std::time::Instant::now();
    // Reading signatures from plugin registry file
    // The plugin.msgpackz file stores the parsed signature collected from each registered plugin
    add_plugin_file(engine_state, plugin_file.clone(), storage_path);
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

                    // Try migration of an old plugin file if this wasn't a custom plugin file
                    if plugin_file.is_none() && migrate_old_plugin_file(engine_state, storage_path)
                    {
                        let Ok(file) = std::fs::File::open(&plugin_path) else {
                            log::warn!("Failed to load newly migrated plugin file");
                            return;
                        };
                        file
                    } else {
                        return;
                    }
                } else {
                    report_error_new(
                        engine_state,
                        &ShellError::GenericError {
                            error: format!(
                                "Error while opening plugin registry file: {}",
                                plugin_path.display()
                            ),
                            msg: "plugin path defined here".into(),
                            span,
                            help: None,
                            inner: vec![err.into()],
                        },
                    );
                    return;
                }
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
        let contents = match PluginRegistryFile::read_from(&mut file, span) {
            Ok(contents) => contents,
            Err(err) => {
                log::warn!("Failed to read plugin registry file: {err:?}");
                report_error_new(
                    engine_state,
                    &ShellError::GenericError {
                        error: format!(
                            "Error while reading plugin registry file: {}",
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

        nu_plugin_engine::load_plugin_file(&mut working_set, &contents, span);

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

    if let Ok(cwd) = engine_state.cwd_as_string(None) {
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
            match engine_state.cwd(Some(stack)) {
                Ok(cwd) => {
                    if let Err(e) = engine_state.merge_env(stack, cwd) {
                        report_error_new(engine_state, &e);
                    }
                }
                Err(e) => {
                    report_error_new(engine_state, &e);
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

#[cfg(feature = "plugin")]
pub fn migrate_old_plugin_file(engine_state: &EngineState, storage_path: &str) -> bool {
    use nu_protocol::{
        PluginExample, PluginIdentity, PluginRegistryItem, PluginRegistryItemData, PluginSignature,
        ShellError,
    };
    use std::collections::BTreeMap;

    let start_time = std::time::Instant::now();

    let Ok(cwd) = engine_state.cwd_as_string(None) else {
        return false;
    };

    let Some(config_dir) = nu_path::config_dir().and_then(|mut dir| {
        dir.push(storage_path);
        nu_path::canonicalize_with(dir, &cwd).ok()
    }) else {
        return false;
    };

    let Ok(old_plugin_file_path) = nu_path::canonicalize_with(OLD_PLUGIN_FILE, &config_dir) else {
        return false;
    };

    let old_contents = match std::fs::read(&old_plugin_file_path) {
        Ok(old_contents) => old_contents,
        Err(err) => {
            report_error_new(
                engine_state,
                &ShellError::GenericError {
                    error: "Can't read old plugin file to migrate".into(),
                    msg: "".into(),
                    span: None,
                    help: Some(err.to_string()),
                    inner: vec![],
                },
            );
            return false;
        }
    };

    // Make a copy of the engine state, because we'll read the newly generated file
    let mut engine_state = engine_state.clone();
    let mut stack = Stack::new();

    if eval_source(
        &mut engine_state,
        &mut stack,
        &old_contents,
        &old_plugin_file_path.to_string_lossy(),
        PipelineData::Empty,
        false,
    ) != 0
    {
        return false;
    }

    // Now that the plugin commands are loaded, we just have to generate the file
    let mut contents = PluginRegistryFile::new();

    let mut groups = BTreeMap::<PluginIdentity, Vec<PluginSignature>>::new();

    for decl in engine_state.plugin_decls() {
        if let Some(identity) = decl.plugin_identity() {
            groups
                .entry(identity.clone())
                .or_default()
                .push(PluginSignature {
                    sig: decl.signature(),
                    examples: decl
                        .examples()
                        .into_iter()
                        .map(PluginExample::from)
                        .collect(),
                })
        }
    }

    for (identity, commands) in groups {
        contents.upsert_plugin(PluginRegistryItem {
            name: identity.name().to_owned(),
            filename: identity.filename().to_owned(),
            shell: identity.shell().map(|p| p.to_owned()),
            data: PluginRegistryItemData::Valid { commands },
        });
    }

    // Write the new file
    let new_plugin_file_path = config_dir.join(PLUGIN_FILE);
    if let Err(err) = std::fs::File::create(&new_plugin_file_path)
        .map_err(|e| e.into())
        .and_then(|file| contents.write_to(file, None))
    {
        report_error_new(
            &engine_state,
            &ShellError::GenericError {
                error: "Failed to save migrated plugin file".into(),
                msg: "".into(),
                span: None,
                help: Some("ensure `$nu.plugin-path` is writable".into()),
                inner: vec![err],
            },
        );
        return false;
    }

    if engine_state.is_interactive {
        eprintln!(
            "Your old plugin.nu file has been migrated to the new format: {}",
            new_plugin_file_path.display()
        );
        eprintln!(
            "The plugin.nu file has not been removed. If `plugin list` looks okay, \
            you may do so manually."
        );
    }

    perf(
        "migrate old plugin file",
        start_time,
        file!(),
        line!(),
        column!(),
        engine_state.get_config().use_ansi_coloring,
    );
    true
}
