use log::info;
#[cfg(feature = "plugin")]
use nu_cli::read_plugin_file;
use nu_cli::{eval_config_contents, eval_source};
use nu_config::ConfigFileKind;
use nu_path::absolute_with;
use nu_protocol::{
    Config, ParseError, PipelineData, Spanned,
    engine::{EngineState, Stack, StateWorkingSet},
    report_parse_error, report_shell_error,
};
use std::{
    fs,
    fs::File,
    io::{Result, Write},
    panic::{AssertUnwindSafe, catch_unwind},
    path::Path,
    sync::Arc,
};

const LOGINSHELL_FILE: &str = "login.nu";

pub(crate) fn read_config_file(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    config_file: Option<Spanned<String>>,
    config_kind: ConfigFileKind,
    create_scaffold: bool,
    strict_mode: bool,
) {
    info!("read_config_file() {config_kind:?} at {config_file:?}",);

    eval_default_config(engine_state, stack, config_kind);

    info!("read_config_file() loading default {config_kind:?}");

    // Load config startup file
    if let Some(file) = config_file {
        match engine_state.cwd_as_string(Some(stack)) {
            Ok(cwd) => {
                if let Ok(path) = absolute_with(&file.item, cwd)
                    && path.exists()
                {
                    eval_config_contents(path, engine_state, stack, strict_mode);
                } else {
                    let e = ParseError::FileNotFound(file.item, file.span);
                    report_parse_error(None, &StateWorkingSet::new(engine_state), &e);
                    if strict_mode {
                        std::process::exit(1);
                    }
                }
            }
            Err(e) => {
                report_shell_error(None, engine_state, &e);
            }
        }
    } else {
        let mut config_path = engine_state.config_dirs.config_home.clone();
        // Create config directory if it does not exist
        if !config_path.exists()
            && let Err(err) = std::fs::create_dir_all(&config_path)
        {
            eprintln!("Failed to create config directory: {err}");
            return;
        }

        config_path.push(config_kind.path());

        if !config_path.exists() {
            let scaffold_config_file = config_kind.scaffold();

            match create_scaffold {
                true => {
                    if let Ok(mut output) = File::create(&config_path) {
                        if write!(output, "{scaffold_config_file}").is_ok() {
                            let config_name = config_kind.name();
                            if engine_state.is_mcp {
                                eprintln!(
                                    "{} file created at: {}",
                                    config_name,
                                    config_path.to_string_lossy()
                                );
                            } else {
                                println!(
                                    "{} file created at: {}",
                                    config_name,
                                    config_path.to_string_lossy()
                                );
                            }
                        } else {
                            eprintln!(
                                "Unable to write to {}, sourcing default file instead",
                                config_path.to_string_lossy(),
                            );
                            return;
                        }
                    } else {
                        eprintln!("Unable to create {scaffold_config_file}");
                        return;
                    }
                }
                _ => {
                    return;
                }
            }
        }

        eval_config_contents(config_path, engine_state, stack, strict_mode);
    }
}

pub(crate) fn read_loginshell_file(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    strict_mode: bool,
) {
    info!(
        "read_loginshell_file() {}:{}:{}",
        file!(),
        line!(),
        column!()
    );

    // read and execute loginshell file if exists
    let mut config_path = engine_state.config_dirs.config_home.clone();
    config_path.push(LOGINSHELL_FILE);

    info!("loginshell_file: {}", config_path.display());

    if config_path.exists() {
        eval_config_contents(config_path, engine_state, stack, strict_mode);
    }
}

pub(crate) fn read_default_env_file(engine_state: &mut EngineState, stack: &mut Stack) {
    let config_file = ConfigFileKind::Env.default();
    eval_source(
        engine_state,
        stack,
        config_file.as_bytes(),
        "default_env.nu",
        PipelineData::empty(),
        false,
    );

    info!(
        "read_default_env_file() env_file_contents: {config_file} {}:{}:{}",
        file!(),
        line!(),
        column!()
    );

    // Merge the environment in case env vars changed in the config
    if let Err(e) = engine_state.merge_env(stack) {
        report_shell_error(None, engine_state, &e);
    }
}

/// Get files sorted lexicographically
///
/// uses `impl Ord for String`
fn read_and_sort_directory(path: &Path) -> Result<Vec<String>> {
    let mut entries = Vec::new();

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let file_name_str = file_name.into_string().unwrap_or_default();
        entries.push(file_name_str);
    }

    entries.sort();

    Ok(entries)
}

pub(crate) fn read_vendor_autoload_files(engine_state: &mut EngineState, stack: &mut Stack) {
    info!(
        "read_vendor_autoload_files() {}:{}:{}",
        file!(),
        line!(),
        column!()
    );

    // Read from the pre-resolved autoload directories (resolved in
    // `nu_config::resolve_paths()` during startup).  Vendor dirs are evaluated
    // first, then user dirs, so users can override vendor autoload files.
    // Clone the dir lists to avoid borrowing engine_state twice (once for the
    // iter and again inside the closure for eval_config_contents).
    let vendor_dirs = engine_state.config_dirs.vendor_autoload_dirs.clone();
    let user_dirs = engine_state.config_dirs.user_autoload_dirs.clone();
    vendor_dirs
        .iter()
        .chain(user_dirs.iter())
        .for_each(|autoload_dir| {
            info!("read_vendor_autoload_files: {}", autoload_dir.display());

            if autoload_dir.exists() {
                // on a second levels files are lexicographically sorted by the string of the filename
                let entries = read_and_sort_directory(autoload_dir);
                if let Ok(entries) = entries {
                    for entry in entries {
                        if !entry.ends_with(".nu") {
                            continue;
                        }
                        let path = autoload_dir.join(entry);
                        info!("AutoLoading: {path:?}");
                        eval_config_contents(path, engine_state, stack, false);
                    }
                }
            }
        });
}

fn eval_default_config(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    config_kind: ConfigFileKind,
) {
    info!("eval_default_config() {config_kind:?}");
    eval_source(
        engine_state,
        stack,
        config_kind.default().as_bytes(),
        config_kind.default_path(),
        PipelineData::empty(),
        false,
    );

    // Merge the environment in case env vars changed in the config
    if let Err(e) = engine_state.merge_env(stack) {
        report_shell_error(Some(stack), engine_state, &e);
    }
}

pub(crate) fn setup_config(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    #[cfg(feature = "plugin")] plugin_file: Option<Spanned<String>>,
    config_file: Option<Spanned<String>>,
    env_file: Option<Spanned<String>>,
    is_login_shell: bool,
) {
    info!(
        "setup_config() config_file_specified: {:?}, env_file_specified: {:?}, login: {}",
        &config_file, &env_file, is_login_shell
    );

    let create_scaffold = !engine_state.config_dirs.config_home.exists();

    let result = catch_unwind(AssertUnwindSafe(|| {
        #[cfg(feature = "plugin")]
        read_plugin_file(engine_state, plugin_file);

        read_config_file(
            engine_state,
            stack,
            env_file,
            ConfigFileKind::Env,
            create_scaffold,
            false,
        );
        read_config_file(
            engine_state,
            stack,
            config_file,
            ConfigFileKind::Config,
            create_scaffold,
            false,
        );

        if is_login_shell {
            read_loginshell_file(engine_state, stack, false);
        }
        // read and auto load vendor autoload files
        read_vendor_autoload_files(engine_state, stack);
    }));
    if result.is_err() {
        eprintln!(
            "A panic occurred while reading configuration files, using default configuration."
        );
        engine_state.config = Arc::new(Config::default())
    }
}
