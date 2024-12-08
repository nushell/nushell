use log::warn;
#[cfg(feature = "plugin")]
use nu_cli::read_plugin_file;
use nu_cli::{eval_config_contents, eval_source};
use nu_engine::convert_env_values;
use nu_path::canonicalize_with;
use nu_protocol::{
    engine::{EngineState, Stack, StateWorkingSet},
    report_parse_error, report_shell_error, report_shell_warning, Config, ParseError, PipelineData, ShellError, Span, Spanned,
};
use nu_utils::{get_default_config, get_default_preconfig, get_scaffold_config, get_scaffold_preconfig, perf};
use std::{
    fs,
    fs::File,
    io::{Result, Write},
    panic::{catch_unwind, AssertUnwindSafe},
    path::Path,
    sync::Arc,
};

const PRECONFIG_FILE: &str = "preconfig.nu";
const ENV_FILE: &str = "env.nu";
const CONFIG_FILE: &str = "config.nu";
const LOGINSHELL_FILE: &str = "login.nu";

pub(crate) fn read_config_file(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    config_file: Option<Spanned<String>>,
    is_preconfig: bool,
    create_scaffold: bool,
) {
    warn!(
        "read_config_file() config_file_specified: {:?}, is_preconfig: {is_preconfig}",
        &config_file
    );

    if is_preconfig {
        eval_default_config(engine_state, stack, get_default_preconfig(), is_preconfig);

        let start_time = std::time::Instant::now();
        let config = engine_state.get_config();
        let use_color = config.use_ansi_coloring;
        // Translate environment variables from Strings to Values
        if let Err(e) = convert_env_values(engine_state, stack) {
            report_shell_error(engine_state, &e);
        }

        perf!(
            "translate env vars after default_preconfig.nu",
            start_time,
            use_color
        );
    } else {
        let start_time = std::time::Instant::now();
        let config = engine_state.get_config();
        let use_color = config.use_ansi_coloring;
        if let Err(e) = convert_env_values(engine_state, stack) {
            report_shell_error(engine_state, &e);
        }
        perf!(
            "translate env vars before default_config.nu",
            start_time,
            use_color
        );

        eval_default_config(engine_state, stack, get_default_config(), is_preconfig);
    };

    warn!("read_config_file() loading_defaults is_preconfig: {is_preconfig}");

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
        if !config_path.exists() {
            if let Err(err) = std::fs::create_dir_all(&config_path) {
                eprintln!("Failed to create config directory: {err}");
                return;
            }
        }

        config_path.push(if is_preconfig { PRECONFIG_FILE } else { CONFIG_FILE });

        if !config_path.exists() {
            let scaffold_config_file = if is_preconfig {
                get_scaffold_preconfig()
            } else {
                get_scaffold_config()
            };

            if create_scaffold {
                if let Ok(mut output) = File::create(&config_path) {
                    if write!(output, "{scaffold_config_file}").is_ok() {
                        let config_type = if is_preconfig {
                            "Preconfig"
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
                        return;
                    }
                } else {
                    eprintln!("Unable to create {scaffold_config_file}");
                    return;
                }
            }
        }

        // Remove this 'if' after env.nu deprecation
        if !config_path.exists() && is_preconfig {
            // If preconfig.nu doesn't exists, try env.nu instead
            if let Some(mut config_path) = nu_path::nu_config_dir() {
                config_path.push(ENV_FILE);
                if config_path.exists() {
                    eval_config_contents(config_path.into(), engine_state, stack);
                    report_shell_warning(
                        engine_state,
                        &ShellError::Deprecated {
                            deprecated: "env.nu",
                            suggestion: "Please use 'preconfig.nu' instead.",
                            span: Span::unknown(),
                            help: None,
                        },
                    );
                }
            }
        } else {
            eval_config_contents(config_path.into(), engine_state, stack);
        }

        // And revert to this original behavior after deprecation
        // eval_config_contents(config_path.into(), engine_state, stack);

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

pub(crate) fn read_default_preconfig_file(engine_state: &mut EngineState, stack: &mut Stack) {
    let config_file = get_default_preconfig();
    eval_source(
        engine_state,
        stack,
        config_file.as_bytes(),
        "default_preconfig.nu",
        PipelineData::empty(),
        false,
    );

    warn!(
        "read_default_preconfig() preconfig_file_contents: {config_file} {}:{}:{}",
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
    for autoload_dir in nu_protocol::eval_const::get_vendor_autoload_dirs(engine_state) {
        warn!("read_vendor_autoload_files: {}", autoload_dir.display());

        if autoload_dir.exists() {
            // on a second levels files are lexicographically sorted by the string of the filename
            let entries = read_and_sort_directory(&autoload_dir);
            if let Ok(entries) = entries {
                for entry in entries {
                    if !entry.ends_with(".nu") {
                        continue;
                    }
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
    is_preconfig: bool,
) {
    warn!("eval_default_config() is_preconfig: {}", is_preconfig);
    eval_source(
        engine_state,
        stack,
        config_file.as_bytes(),
        if is_preconfig {
            "default_preconfig.nu"
        } else {
            "default_config.nu"
        },
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
    preconfig_file: Option<Spanned<String>>,
    is_login_shell: bool,
) {
    warn!(
        "setup_config() config_file_specified: {:?}, preconfig_file_specified: {:?}, login: {}",
        &config_file, &preconfig_file, is_login_shell
    );

    let create_scaffold = nu_path::nu_config_dir().map_or(false, |p| !p.exists());

    let result = catch_unwind(AssertUnwindSafe(|| {
        #[cfg(feature = "plugin")]
        read_plugin_file(engine_state, plugin_file);

        read_config_file(engine_state, stack, preconfig_file, true, false);
        read_config_file(engine_state, stack, config_file, false, create_scaffold);

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
