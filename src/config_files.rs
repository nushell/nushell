use log::info;
#[cfg(feature = "plugin")]
use nu_cli::read_plugin_file;
use nu_cli::{eval_config_contents, eval_source};
use nu_config::ConfigFileKind;
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

/// Load a config/env file from the already-resolved path in `config_dirs`.
///
/// Paths come only from `engine_state.config_dirs` — never re-resolved.
/// When the path is a CLI override (`ConfigPath::Override`), a missing file is
/// reported as an error. `cli_override` is the original CLI path string/span
/// used only for error messages (so the user sees the path they typed, not the
/// absolute form). Otherwise first-run scaffolding may create the default file
/// under `config_home`.
pub(crate) fn read_config_file(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    config_kind: ConfigFileKind,
    create_scaffold: bool,
    strict_mode: bool,
    cli_override: Option<&Spanned<String>>,
) {
    info!("read_config_file() {config_kind:?}");

    eval_default_config(engine_state, stack, config_kind);

    info!("read_config_file() loading default {config_kind:?}");

    let resolved = match config_kind {
        ConfigFileKind::Config => &engine_state.config_dirs.config_file,
        ConfigFileKind::Env => &engine_state.config_dirs.env_file,
    };
    let is_override = resolved.is_override();
    let config_path = resolved.to_path_buf();

    if is_override {
        if config_path.exists() {
            eval_config_contents(config_path, engine_state, stack, strict_mode);
        } else {
            // Prefer the original CLI path string for the error (matches historical
            // behavior and tests). Fall back to the resolved absolute path.
            let (display_path, span) = match cli_override {
                Some(s) => (s.item.clone(), s.span),
                None => (
                    config_path.display().to_string(),
                    nu_protocol::Span::unknown(),
                ),
            };
            let e = ParseError::FileNotFound(display_path, span);
            report_parse_error(None, &StateWorkingSet::new(engine_state), &e);
            if strict_mode {
                std::process::exit(1);
            }
        }
        return;
    }

    // Default path under config_home — may scaffold on first run.
    let mut config_dir = engine_state.config_dirs.config_home.clone();
    if !config_dir.exists()
        && let Err(err) = std::fs::create_dir_all(&config_dir)
    {
        eprintln!("Failed to create config directory: {err}");
        return;
    }

    // Prefer the resolved path; fall back to config_home + kind if empty.
    let config_path = if config_path.as_os_str().is_empty() {
        config_dir.push(config_kind.path());
        config_dir
    } else {
        config_path
    };

    if !config_path.exists() {
        let scaffold_config_file = config_kind.scaffold();
        if !create_scaffold {
            return;
        }

        let Ok(mut output) = File::create(&config_path) else {
            return eprintln!("Unable to create {scaffold_config_file}");
        };

        if write!(output, "{scaffold_config_file}").is_err() {
            return eprintln!(
                "Unable to write to {}, sourcing default file instead",
                config_path.to_string_lossy(),
            );
        }

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
    }

    eval_config_contents(config_path, engine_state, stack, strict_mode);
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
    // `nu_config::resolve_paths()` during startup). Vendor dirs are evaluated
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
    is_login_shell: bool,
) {
    info!("setup_config() login: {is_login_shell}");

    let create_scaffold = !engine_state.config_dirs.config_home.exists();

    let result = catch_unwind(AssertUnwindSafe(|| {
        #[cfg(feature = "plugin")]
        read_plugin_file(engine_state, None);

        read_config_file(
            engine_state,
            stack,
            ConfigFileKind::Env,
            create_scaffold,
            false,
            None,
        );
        read_config_file(
            engine_state,
            stack,
            ConfigFileKind::Config,
            create_scaffold,
            false,
            None,
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
