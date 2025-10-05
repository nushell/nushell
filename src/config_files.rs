use log::warn;
#[cfg(feature = "plugin")]
use nu_cli::read_plugin_file;
use nu_cli::{eval_config_contents, eval_source};
use nu_path::canonicalize_with;
use nu_protocol::{
    Config, ParseError, PipelineData, Spanned,
    engine::{EngineState, Stack, StateWorkingSet},
    eval_const::{get_user_autoload_dirs, get_vendor_autoload_dirs},
    report_parse_error, report_shell_error,
};
use nu_utils::ConfigFileKind;
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
) {
    warn!("read_config_file() {config_kind:?} at {config_file:?}",);

    eval_default_config(engine_state, stack, config_kind);

    warn!("read_config_file() loading default {config_kind:?}");

    // Load config startup file
    if let Some(file) = config_file {
        match engine_state.cwd_as_string(Some(stack)) {
            Ok(cwd) => {
                if let Ok(path) = canonicalize_with(&file.item, cwd) {
                    eval_config_contents(path, engine_state, stack);
                } else {
                    let e = ParseError::FileNotFound(file.item, file.span);
                    report_parse_error(&StateWorkingSet::new(engine_state), &e);
                }
            }
            Err(e) => {
                report_shell_error(engine_state, &e);
            }
        }
    } else if let Some(mut config_path) = nu_path::nu_config_dir() {
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
                            println!(
                                "{} file created at: {}",
                                config_name,
                                config_path.to_string_lossy()
                            );
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

        eval_config_contents(config_path.into(), engine_state, stack);
    }
}

pub(crate) fn read_loginshell_file(engine_state: &mut EngineState, stack: &mut Stack) {
    warn!(
        "read_loginshell_file() {}:{}:{}",
        file!(),
        line!(),
        column!()
    );

    // read and execute loginshell file if exists
    if let Some(mut config_path) = nu_path::nu_config_dir() {
        config_path.push(LOGINSHELL_FILE);

        warn!("loginshell_file: {}", config_path.display());

        if config_path.exists() {
            eval_config_contents(config_path.into(), engine_state, stack);
        }
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

    warn!(
        "read_default_env_file() env_file_contents: {config_file} {}:{}:{}",
        file!(),
        line!(),
        column!()
    );

    // Merge the environment in case env vars changed in the config
    if let Err(e) = engine_state.merge_env(stack) {
        report_shell_error(engine_state, &e);
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
    warn!(
        "read_vendor_autoload_files() {}:{}:{}",
        file!(),
        line!(),
        column!()
    );

    // The evaluation order is first determined by the semantics of `get_vendor_autoload_dirs`
    // to determine the order of directories to evaluate
    get_vendor_autoload_dirs(engine_state)
        .iter()
        // User autoload directories are evaluated after vendor, which means that
        // the user can override vendor autoload files
        .chain(get_user_autoload_dirs(engine_state).iter())
        .for_each(|autoload_dir| {
            warn!("read_vendor_autoload_files: {}", autoload_dir.display());

            if autoload_dir.exists() {
                // on a second levels files are lexicographically sorted by the string of the filename
                let entries = read_and_sort_directory(autoload_dir);
                if let Ok(entries) = entries {
                    for entry in entries {
                        if !entry.ends_with(".nu") {
                            continue;
                        }
                        let path = autoload_dir.join(entry);
                        warn!("AutoLoading: {path:?}");
                        eval_config_contents(path, engine_state, stack);
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
    warn!("eval_default_config() {config_kind:?}");
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
        report_shell_error(engine_state, &e);
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
    warn!(
        "setup_config() config_file_specified: {:?}, env_file_specified: {:?}, login: {}",
        &config_file, &env_file, is_login_shell
    );

    let create_scaffold = nu_path::nu_config_dir().is_some_and(|p| !p.exists());

    let result = catch_unwind(AssertUnwindSafe(|| {
        #[cfg(feature = "plugin")]
        read_plugin_file(engine_state, plugin_file);

        read_config_file(
            engine_state,
            stack,
            env_file,
            ConfigFileKind::Env,
            create_scaffold,
        );
        read_config_file(
            engine_state,
            stack,
            config_file,
            ConfigFileKind::Config,
            create_scaffold,
        );

        if is_login_shell {
            read_loginshell_file(engine_state, stack);
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

pub(crate) fn set_config_path(
    engine_state: &mut EngineState,
    cwd: &Path,
    default_config_name: &str,
    key: &str,
    config_file: Option<&Spanned<String>>,
) {
    warn!(
        "set_config_path() cwd: {:?}, default_config: {}, key: {}, config_file_specified: {:?}",
        &cwd, &default_config_name, &key, &config_file
    );
    let config_path = match config_file {
        Some(s) => canonicalize_with(&s.item, cwd).ok(),
        None => nu_path::nu_config_dir().map(|p| {
            let mut p = canonicalize_with(&p, cwd).unwrap_or(p.into());
            p.push(default_config_name);
            canonicalize_with(&p, cwd).unwrap_or(p)
        }),
    };

    if let Some(path) = config_path {
        engine_state.set_config_path(key, path);
    }
}
