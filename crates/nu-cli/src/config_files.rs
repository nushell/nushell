use crate::util::eval_source;
#[cfg(feature = "plugin")]
use nu_path::absolute_with;
#[cfg(feature = "plugin")]
use nu_protocol::shell_error::generic::GenericError;
#[cfg(feature = "plugin")]
use nu_protocol::{ParseError, PluginRegistryFile, Span, engine::StateWorkingSet};
use nu_protocol::{
    PipelineData,
    engine::{EngineState, Stack},
    report_shell_error,
};
#[cfg(feature = "plugin")]
use nu_utils::perf;
#[cfg(feature = "plugin")]
use nu_utils::time::Instant;
use std::path::PathBuf;

#[cfg(feature = "plugin")]
const PLUGIN_FILE: &str = "plugin.msgpackz";
#[cfg(feature = "plugin")]
const OLD_PLUGIN_FILE: &str = "plugin.nu";

/// Load the plugin registry file from the already-resolved path in
/// `engine_state.config_dirs.plugin_file`.
///
/// `override_span` is the CLI span for `--plugin-config` when provided, used
/// only for error reporting.
#[cfg(feature = "plugin")]
pub fn read_plugin_file(engine_state: &mut EngineState, override_span: Option<Span>) {
    use nu_protocol::{ShellError, shell_error::io::IoError};

    let span = override_span;
    let is_override = engine_state.config_dirs.plugin_file.is_override();

    // Check and warn + abort if this is a .nu plugin file
    if engine_state
        .config_dirs
        .plugin_file
        .as_path()
        .extension()
        .is_some_and(|ext| ext == "nu")
    {
        let error = "Wrong plugin file format";
        let msg = ".nu plugin files are no longer supported";
        report_shell_error(
            None,
            engine_state,
            &ShellError::Generic(
                match span {
                    Some(span) => GenericError::new(error, msg, span),
                    None => GenericError::new_internal(error, msg),
                }
                .with_help("please recreate this file in the new .msgpackz format"),
            ),
        );
        return;
    }

    let mut start_time = Instant::now();
    // Sync engine_state.plugin_path from the single resolved config_dirs path.
    add_plugin_file(engine_state, override_span);
    perf!(
        "add plugin file to engine_state",
        start_time,
        engine_state
            .get_config()
            .use_ansi_coloring
            .get(engine_state)
    );

    start_time = Instant::now();
    let plugin_path = engine_state.plugin_path.clone();
    if let Some(plugin_path) = plugin_path {
        // Open the plugin file
        let mut file = match std::fs::File::open(&plugin_path) {
            Ok(file) => file,
            Err(err) => {
                if err.kind() == std::io::ErrorKind::NotFound {
                    log::warn!("Plugin file not found: {}", plugin_path.display());

                    // Try migration of an old plugin file if this wasn't a custom plugin file
                    if !is_override && migrate_old_plugin_file(engine_state) {
                        let Ok(file) = std::fs::File::open(&plugin_path) else {
                            log::warn!("Failed to load newly migrated plugin file");
                            return;
                        };
                        file
                    } else {
                        return;
                    }
                } else {
                    report_shell_error(
                        None,
                        engine_state,
                        &ShellError::Io(IoError::new_internal_with_path(
                            err,
                            "Could not open plugin registry file",
                            plugin_path,
                        )),
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
                let error = format!(
                    "Error while reading plugin registry file: {}",
                    plugin_path.display()
                );
                let msg = "plugin path defined here";
                report_shell_error(
                    None,
                    engine_state,
                    &ShellError::Generic(
                        match span {
                            Some(span) => GenericError::new(error, msg, span),
                            None => GenericError::new_internal(error, msg),
                        }
                        .with_help(
                            "you might try deleting the file and registering all of your plugins again",
                        ),
                    ),
                );
                return;
            }
        };

        perf!(
            &format!("read plugin file {}", plugin_path.display()),
            start_time,
            engine_state
                .get_config()
                .use_ansi_coloring
                .get(engine_state)
        );
        start_time = Instant::now();

        let mut working_set = StateWorkingSet::new(engine_state);

        let plugin_load_errors =
            nu_plugin_engine::load_plugin_file(&mut working_set, &contents, span);

        if plugin_load_errors > 0 {
            let error = format!(
                "Failed to load {plugin_load_errors} plugin entr{} from {}",
                if plugin_load_errors == 1 { "y" } else { "ies" },
                plugin_path.display(),
            );
            let msg = "plugins with incompatible or invalid registry data were skipped";
            let help = "run `plugin list` and re-add outdated plugins with `plugin add`";
            let generic_error = match span {
                Some(span) => GenericError::new(error, msg, span),
                None => GenericError::new_internal(error, msg),
            };
            report_shell_error(
                None,
                engine_state,
                &ShellError::Generic(generic_error.with_help(help)),
            );
        }

        if let Err(err) = engine_state.merge_delta(working_set.render()) {
            report_shell_error(None, engine_state, &err);
            return;
        }

        perf!(
            &format!("load plugin file {}", plugin_path.display()),
            start_time,
            engine_state
                .get_config()
                .use_ansi_coloring
                .get(engine_state)
        );
    }
}

/// Ensure `engine_state.plugin_path` matches the resolved
/// `config_dirs.plugin_file`.
///
/// For CLI overrides, reports an error if the parent directory does not exist.
#[cfg(feature = "plugin")]
pub fn add_plugin_file(engine_state: &mut EngineState, override_span: Option<Span>) {
    use std::path::Path;

    use nu_protocol::report_parse_error;

    let plugin_path = engine_state.config_dirs.plugin_file.to_path_buf();
    if plugin_path.as_os_str().is_empty() {
        return;
    }

    let Ok(cwd) = engine_state.cwd_as_string(None) else {
        return;
    };

    if engine_state.config_dirs.plugin_file.is_override() {
        let path = Path::new(&plugin_path);
        let path_dir = path.parent().unwrap_or(path);
        if let Ok(path_dir) = absolute_with(path_dir, &cwd)
            && path_dir.exists()
        {
            let path = path_dir.join(path.file_name().unwrap_or(path.as_os_str()));
            let path = absolute_with(&path, &cwd).unwrap_or(path);
            engine_state.plugin_path = Some(path);
        } else {
            report_parse_error(
                None,
                &StateWorkingSet::new(engine_state),
                &ParseError::FileNotFound(
                    path_dir.to_string_lossy().into_owned(),
                    override_span.unwrap_or_else(Span::unknown),
                ),
            );
        }
    } else {
        // Default registry path — already resolved; just absolute-ize for cwd.
        let plugin_path = absolute_with(&plugin_path, &cwd).unwrap_or(plugin_path);
        engine_state.plugin_path = Some(plugin_path);
    }
}

pub fn eval_config_contents(
    config_path: PathBuf,
    engine_state: &mut EngineState,
    stack: &mut Stack,
    strict_mode: bool,
) {
    if config_path.exists() & config_path.is_file() {
        let config_filename = config_path.to_string_lossy();

        if let Ok(contents) = std::fs::read(&config_path) {
            // Set the current active file to the config file.
            let prev_file = engine_state.file.take();
            engine_state.file = Some(config_path.clone());

            // TODO: ignore this error?
            let exit_code = eval_source(
                engine_state,
                stack,
                &contents,
                &config_filename,
                PipelineData::empty(),
                false,
            );
            if exit_code != 0 && strict_mode {
                std::process::exit(exit_code)
            }

            // Restore the current active file.
            engine_state.file = prev_file;

            // Merge the environment in case env vars changed in the config
            if let Err(e) = engine_state.merge_env(stack) {
                report_shell_error(Some(stack), engine_state, &e);
            }
        }
    }
}

#[cfg(feature = "plugin")]
pub fn migrate_old_plugin_file(engine_state: &EngineState) -> bool {
    use nu_protocol::{
        PluginExample, PluginIdentity, PluginRegistryItem, PluginRegistryItemData, PluginSignature,
        ShellError, shell_error::io::IoError,
    };
    use std::collections::BTreeMap;

    let start_time = Instant::now();

    let config_dir = &engine_state.config_dirs.config_home;
    if config_dir.as_os_str().is_empty() {
        return false;
    }

    let Ok(old_plugin_file_path) = nu_path::absolute_with(OLD_PLUGIN_FILE, config_dir) else {
        return false;
    };

    if !config_dir.exists() || !old_plugin_file_path.exists() {
        return false;
    }

    let old_contents = match std::fs::read(&old_plugin_file_path) {
        Ok(old_contents) => old_contents,
        Err(err) => {
            report_shell_error(
                None,
                engine_state,
                &ShellError::Generic(
                    GenericError::new_internal("Can't read old plugin file to migrate", "")
                        .with_help(err.to_string()),
                ),
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
        PipelineData::empty(),
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
            data: PluginRegistryItemData::Valid {
                metadata: Default::default(),
                commands,
            },
        });
    }

    // Write the new file
    let new_plugin_file_path = config_dir.join(PLUGIN_FILE);
    if let Err(err) = std::fs::File::create(&new_plugin_file_path)
        .map_err(|err| {
            IoError::new_internal_with_path(
                err,
                "Could not create new plugin file",
                new_plugin_file_path.clone(),
            )
        })
        .map_err(ShellError::from)
        .and_then(|file| contents.write_to(file, None))
    {
        report_shell_error(
            None,
            &engine_state,
            &ShellError::Generic(
                GenericError::new_internal("Failed to save migrated plugin file", "")
                    .with_help("ensure `$nu.plugin-path` is writable")
                    .with_inner([err]),
            ),
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

    perf!(
        "migrate old plugin file",
        start_time,
        engine_state
            .get_config()
            .use_ansi_coloring
            .get(&engine_state)
    );
    true
}
