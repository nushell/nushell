use log::warn;
#[cfg(feature = "plugin")]
use nu_cli::read_plugin_file;
use nu_cli::{eval_config_contents, eval_source};
use nu_path::canonicalize_with;
use nu_protocol::{
    engine::{EngineState, Stack, StateWorkingSet},
    report_error, report_error_new, Config, ParseError, PipelineData, Spanned,
};
use nu_utils::{get_default_config, get_default_env};
use std::{
    fs,
    fs::File,
    io::{Result, Write},
    panic::{catch_unwind, AssertUnwindSafe},
    path::Path,
    sync::Arc,
};

pub(crate) const NUSHELL_FOLDER: &str = "nushell";
const CONFIG_FILE: &str = "config.nu";
const ENV_FILE: &str = "env.nu";
const LOGINSHELL_FILE: &str = "login.nu";

pub(crate) fn read_config_file(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    config_file: Option<Spanned<String>>,
    is_env_config: bool,
) {
    warn!(
        "read_config_file() config_file_specified: {:?}, is_env_config: {is_env_config}",
        &config_file
    );
    // Load config startup file
    if let Some(file) = config_file {
        let working_set = StateWorkingSet::new(engine_state);

        match engine_state.cwd_as_string(Some(stack)) {
            Ok(cwd) => {
                if let Ok(path) = canonicalize_with(&file.item, cwd) {
                    eval_config_contents(path, engine_state, stack);
                } else {
                    let e = ParseError::FileNotFound(file.item, file.span);
                    report_error(&working_set, &e);
                }
            }
            Err(e) => {
                report_error(&working_set, &e);
            }
        }
    } else if let Some(mut config_path) = nu_path::config_dir() {
        config_path.push(NUSHELL_FOLDER);

        // Create config directory if it does not exist
        if !config_path.exists() {
            if let Err(err) = std::fs::create_dir_all(&config_path) {
                eprintln!("Failed to create config directory: {err}");
                return;
            }
        }

        config_path.push(if is_env_config { ENV_FILE } else { CONFIG_FILE });

        if !config_path.exists() {
            let file_msg = if is_env_config {
                "environment config"
            } else {
                "config"
            };
            println!(
                "No {} file found at {}",
                file_msg,
                config_path.to_string_lossy()
            );
            println!("Would you like to create one with defaults (Y/n): ");

            let mut answer = String::new();
            std::io::stdin()
                .read_line(&mut answer)
                .expect("Failed to read user input");

            let config_file = if is_env_config {
                get_default_env()
            } else {
                get_default_config()
            };

            match answer.trim() {
                "y" | "Y" | "" => {
                    if let Ok(mut output) = File::create(&config_path) {
                        if write!(output, "{config_file}").is_ok() {
                            let config_type = if is_env_config {
                                "Environment config"
                            } else {
                                "Config"
                            };
                            println!(
                                "{} file created at: {}",
                                config_type,
                                config_path.to_string_lossy()
                            );
                        } else {
                            eprintln!(
                                "Unable to write to {}, sourcing default file instead",
                                config_path.to_string_lossy(),
                            );
                            eval_default_config(engine_state, stack, config_file, is_env_config);
                            return;
                        }
                    } else {
                        eprintln!("Unable to create {config_file}, sourcing default file instead");
                        eval_default_config(engine_state, stack, config_file, is_env_config);
                        return;
                    }
                }
                _ => {
                    eval_default_config(engine_state, stack, config_file, is_env_config);
                    return;
                }
            }
        }

        eval_config_contents(config_path, engine_state, stack);
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
    if let Some(mut config_path) = nu_path::config_dir() {
        config_path.push(NUSHELL_FOLDER);
        config_path.push(LOGINSHELL_FILE);

        warn!("loginshell_file: {}", config_path.display());

        if config_path.exists() {
            eval_config_contents(config_path, engine_state, stack);
        }
    }
}

pub(crate) fn read_default_env_file(engine_state: &mut EngineState, stack: &mut Stack) {
    let config_file = get_default_env();
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

    // read and source vendor_autoload_files file if exists
    if let Some(autoload_dir) = nu_protocol::eval_const::get_vendor_autoload_dir(engine_state) {
        warn!("read_vendor_autoload_files: {}", autoload_dir.display());

        if autoload_dir.exists() {
            let entries = read_and_sort_directory(&autoload_dir);
            if let Ok(entries) = entries {
                for entry in entries {
                    let path = autoload_dir.join(entry);
                    warn!("AutoLoading: {:?}", path);
                    eval_config_contents(path, engine_state, stack);
                }
            }
        }
    }
}

fn eval_default_config(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    config_file: &str,
    is_env_config: bool,
) {
    warn!(
        "eval_default_config() config_file_specified: {:?}, is_env_config: {}",
        &config_file, is_env_config
    );
    println!("Continuing without config file");
    // Just use the contents of "default_config.nu" or "default_env.nu"
    eval_source(
        engine_state,
        stack,
        config_file.as_bytes(),
        if is_env_config {
            "default_env.nu"
        } else {
            "default_config.nu"
        },
        PipelineData::empty(),
        false,
    );

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
    let result = catch_unwind(AssertUnwindSafe(|| {
        #[cfg(feature = "plugin")]
        read_plugin_file(engine_state, plugin_file, NUSHELL_FOLDER);

        read_config_file(engine_state, stack, env_file, true);
        read_config_file(engine_state, stack, config_file, false);

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
        None => nu_path::config_dir().map(|mut p| {
            p.push(NUSHELL_FOLDER);
            let mut p = canonicalize_with(&p, cwd).unwrap_or(p);
            p.push(default_config_name);
            canonicalize_with(&p, cwd).unwrap_or(p)
        }),
    };

    if let Some(path) = config_path {
        engine_state.set_config_path(key, path);
    }
}
