mod command;
mod command_context;
mod config_files;
mod experimental_options;
mod ide;
mod logger;
mod run;
mod signals;
#[cfg(unix)]
mod terminal;
mod test_bins;

use crate::{
    command::parse_commandline_args,
    config_files::set_config_path,
    logger::{configure, logger},
};
use command::gather_commandline_args;
use log::{Level, trace};
use miette::Result;
use nu_cli::gather_parent_env_vars;
use nu_engine::{convert_env_values, exit::cleanup_exit};
use nu_lsp::LanguageServer;
use nu_path::canonicalize_with;
use nu_protocol::{
    ByteStream, Config, IntoValue, PipelineData, ShellError, Span, Spanned, Type, Value,
    engine::{EngineState, Stack},
    record, report_shell_error,
};
use nu_std::load_standard_library;
use nu_utils::perf;
use run::{run_commands, run_file, run_repl};
use signals::ctrlc_protection;
use std::{borrow::Cow, path::PathBuf, str::FromStr, sync::Arc};

/// Get the directory where the Nushell executable is located.
fn current_exe_directory() -> PathBuf {
    let mut path = std::env::current_exe().expect("current_exe() should succeed");
    path.pop();
    path
}

/// Get the current working directory from the environment.
fn current_dir_from_environment() -> PathBuf {
    if let Ok(cwd) = std::env::current_dir() {
        return cwd;
    }
    if let Ok(cwd) = std::env::var("PWD") {
        return cwd.into();
    }
    if let Some(home) = nu_path::home_dir() {
        return home.into_std_path_buf();
    }
    current_exe_directory()
}

fn main() -> Result<()> {
    let entire_start_time = std::time::Instant::now();
    let mut start_time = std::time::Instant::now();
    miette::set_panic_hook();
    let miette_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |x| {
        crossterm::terminal::disable_raw_mode().expect("unable to disable raw mode");
        miette_hook(x);
    }));

    let mut engine_state = EngineState::new();

    // Parse commandline args very early and load experimental options to allow loading different
    // commands based on experimental options.
    let (args_to_nushell, script_name, args_to_script) = gather_commandline_args();
    let parsed_nu_cli_args = parse_commandline_args(&args_to_nushell.join(" "), &mut engine_state)
        .unwrap_or_else(|err| {
            report_shell_error(&engine_state, &err);
            std::process::exit(1)
        });

    experimental_options::load(&engine_state, &parsed_nu_cli_args, !script_name.is_empty());

    let mut engine_state = command_context::add_command_context(engine_state);

    // Provide `version` the features of this nu binary
    let cargo_features = env!("NU_FEATURES").split(",").map(Cow::Borrowed).collect();
    nu_cmd_lang::VERSION_NU_FEATURES
        .set(cargo_features)
        .expect("unable to set VERSION_NU_FEATURES");

    // Get the current working directory from the environment.
    let init_cwd = current_dir_from_environment();

    // Custom additions
    let delta = {
        let mut working_set = nu_protocol::engine::StateWorkingSet::new(&engine_state);
        working_set.add_decl(Box::new(nu_cli::NuHighlight));
        working_set.add_decl(Box::new(nu_cli::Print));
        working_set.render()
    };

    if let Err(err) = engine_state.merge_delta(delta) {
        report_shell_error(&engine_state, &err);
    }

    // TODO: make this conditional in the future
    ctrlc_protection(&mut engine_state);

    #[cfg(all(feature = "rustls-tls", feature = "network"))]
    nu_command::tls::CRYPTO_PROVIDER.default();

    // Begin: Default NU_LIB_DIRS, NU_PLUGIN_DIRS
    // Set default NU_LIB_DIRS and NU_PLUGIN_DIRS here before the env.nu is processed. If
    // the env.nu file exists, these values will be overwritten, if it does not exist, or
    // there is an error reading it, these values will be used.
    let nushell_config_path: PathBuf = nu_path::nu_config_dir().map(Into::into).unwrap_or_default();
    if let Ok(xdg_config_home) = std::env::var("XDG_CONFIG_HOME")
        && !xdg_config_home.is_empty()
    {
        if nushell_config_path
            != canonicalize_with(&xdg_config_home, &init_cwd)
                .unwrap_or(PathBuf::from(&xdg_config_home))
                .join("nushell")
        {
            report_shell_error(
                &engine_state,
                &ShellError::InvalidXdgConfig {
                    xdg: xdg_config_home,
                    default: nushell_config_path.display().to_string(),
                },
            );
        } else if let Some(old_config) = dirs::config_dir()
            .and_then(|p| p.canonicalize().ok())
            .map(|p| p.join("nushell"))
        {
            let xdg_config_empty = nushell_config_path
                .read_dir()
                .map_or(true, |mut dir| dir.next().is_none());
            let old_config_empty = old_config
                .read_dir()
                .map_or(true, |mut dir| dir.next().is_none());
            if !old_config_empty && xdg_config_empty {
                eprintln!(
                    "WARNING: XDG_CONFIG_HOME has been set but {} is empty.\n",
                    nushell_config_path.display(),
                );
                eprintln!(
                    "Nushell will not move your configuration files from {}",
                    old_config.display()
                );
            }
        }
    }

    let default_nushell_completions_path = if let Some(mut path) = nu_path::data_dir() {
        path.push("nushell");
        path.push("completions");
        path.into()
    } else {
        std::path::PathBuf::new()
    };

    let mut default_nu_lib_dirs_path = nushell_config_path.clone();
    default_nu_lib_dirs_path.push("scripts");
    // env.NU_LIB_DIRS to be replaced by constant (below) - Eventual deprecation
    // but an empty list for now to allow older code to work
    engine_state.add_env_var("NU_LIB_DIRS".to_string(), Value::test_list(vec![]));

    let mut working_set = nu_protocol::engine::StateWorkingSet::new(&engine_state);
    let var_id = working_set.add_variable(
        b"$NU_LIB_DIRS".into(),
        Span::unknown(),
        Type::List(Box::new(Type::String)),
        false,
    );
    working_set.set_variable_const_val(
        var_id,
        Value::test_list(vec![
            Value::test_string(default_nu_lib_dirs_path.to_string_lossy()),
            Value::test_string(default_nushell_completions_path.to_string_lossy()),
        ]),
    );
    engine_state.merge_delta(working_set.render())?;

    let mut default_nu_plugin_dirs_path = nushell_config_path;
    default_nu_plugin_dirs_path.push("plugins");
    engine_state.add_env_var("NU_PLUGIN_DIRS".to_string(), Value::test_list(vec![]));
    let mut working_set = nu_protocol::engine::StateWorkingSet::new(&engine_state);
    let var_id = working_set.add_variable(
        b"$NU_PLUGIN_DIRS".into(),
        Span::unknown(),
        Type::List(Box::new(Type::String)),
        false,
    );
    working_set.set_variable_const_val(
        var_id,
        Value::test_list(vec![
            Value::test_string(default_nu_plugin_dirs_path.to_string_lossy()),
            Value::test_string(current_exe_directory().to_string_lossy()),
        ]),
    );
    engine_state.merge_delta(working_set.render())?;
    // End: Default NU_LIB_DIRS, NU_PLUGIN_DIRS

    // This is the real secret sauce to having an in-memory sqlite db. You must
    // start a connection to the memory database in main so it will exist for the
    // lifetime of the program. If it's created with how MEMORY_DB is defined
    // you'll be able to access this open connection from anywhere in the program
    // by using the identical connection string.
    #[cfg(feature = "sqlite")]
    let db = nu_command::open_connection_in_memory_custom()?;
    #[cfg(feature = "sqlite")]
    db.last_insert_rowid();

    // keep this condition in sync with the branches at the end
    engine_state.is_interactive = parsed_nu_cli_args.interactive_shell.is_some()
        || (parsed_nu_cli_args.testbin.is_none()
            && parsed_nu_cli_args.commands.is_none()
            && script_name.is_empty()
            && !parsed_nu_cli_args.lsp);

    engine_state.is_login = parsed_nu_cli_args.login_shell.is_some();
    engine_state.history_enabled = parsed_nu_cli_args.no_history.is_none();
    engine_state.is_lsp = parsed_nu_cli_args.lsp;

    let use_color = engine_state
        .get_config()
        .use_ansi_coloring
        .get(&engine_state);

    // Set up logger
    if let Some(level) = parsed_nu_cli_args
        .log_level
        .as_ref()
        .map(|level| level.item.clone())
    {
        let level = if Level::from_str(&level).is_ok() {
            level
        } else {
            eprintln!(
                "ERROR: log library did not recognize log level '{level}', using default 'info'"
            );
            "info".to_string()
        };
        let target = parsed_nu_cli_args
            .log_target
            .as_ref()
            .map(|target| target.item.clone())
            .unwrap_or_else(|| "stderr".to_string());

        let make_filters = |filters: &Option<Vec<Spanned<String>>>| {
            filters.as_ref().map(|filters| {
                filters
                    .iter()
                    .map(|filter| filter.item.clone())
                    .collect::<Vec<String>>()
            })
        };
        let filters = logger::Filters {
            include: make_filters(&parsed_nu_cli_args.log_include),
            exclude: make_filters(&parsed_nu_cli_args.log_exclude),
        };

        logger(|builder| configure(&level, &target, filters, builder))?;
        // info!("start logging {}:{}:{}", file!(), line!(), column!());
        perf!("start logging", start_time, use_color);
    }

    start_time = std::time::Instant::now();
    set_config_path(
        &mut engine_state,
        init_cwd.as_ref(),
        "config.nu",
        "config-path",
        parsed_nu_cli_args.config_file.as_ref(),
    );

    set_config_path(
        &mut engine_state,
        init_cwd.as_ref(),
        "env.nu",
        "env-path",
        parsed_nu_cli_args.env_file.as_ref(),
    );
    perf!("set_config_path", start_time, use_color);

    #[cfg(unix)]
    {
        start_time = std::time::Instant::now();
        terminal::acquire(engine_state.is_interactive);
        perf!("acquire_terminal", start_time, use_color);
    }

    start_time = std::time::Instant::now();
    engine_state.add_env_var(
        "config".into(),
        Config::default().into_value(Span::unknown()),
    );
    perf!("$env.config setup", start_time, use_color);

    engine_state.add_env_var(
        "ENV_CONVERSIONS".to_string(),
        Value::test_record(record! {}),
    );

    start_time = std::time::Instant::now();
    if let Some(include_path) = &parsed_nu_cli_args.include_path {
        let span = include_path.span;
        let vals: Vec<_> = include_path
            .item
            .split('\x1e') // \x1e is the record separator character (a character that is unlikely to appear in a path)
            .map(|x| Value::string(x.trim().to_string(), span))
            .collect();

        let mut working_set = nu_protocol::engine::StateWorkingSet::new(&engine_state);
        let var_id = working_set.add_variable(
            b"$NU_LIB_DIRS".into(),
            span,
            Type::List(Box::new(Type::String)),
            false,
        );
        working_set.set_variable_const_val(var_id, Value::list(vals, span));
        engine_state.merge_delta(working_set.render())?;
    }
    perf!("NU_LIB_DIRS setup", start_time, use_color);

    start_time = std::time::Instant::now();
    // First, set up env vars as strings only
    gather_parent_env_vars(&mut engine_state, init_cwd.as_ref());
    perf!("gather env vars", start_time, use_color);

    let mut stack = Stack::new();
    start_time = std::time::Instant::now();
    let config = engine_state.get_config();
    let use_color = config.use_ansi_coloring.get(&engine_state);
    // Translate environment variables from Strings to Values
    if let Err(e) = convert_env_values(&mut engine_state, &mut stack) {
        report_shell_error(&engine_state, &e);
    }
    perf!("Convert path to list", start_time, use_color);

    engine_state.add_env_var(
        "NU_VERSION".to_string(),
        Value::string(env!("CARGO_PKG_VERSION"), Span::unknown()),
    );

    if parsed_nu_cli_args.no_std_lib.is_none() {
        load_standard_library(&mut engine_state)?;
    }

    // IDE commands
    if let Some(ide_goto_def) = parsed_nu_cli_args.ide_goto_def {
        ide::goto_def(&mut engine_state, &script_name, &ide_goto_def);

        return Ok(());
    } else if let Some(ide_hover) = parsed_nu_cli_args.ide_hover {
        ide::hover(&mut engine_state, &script_name, &ide_hover);

        return Ok(());
    } else if let Some(ide_complete) = parsed_nu_cli_args.ide_complete {
        let cwd = std::env::current_dir().expect("Could not get current working directory.");
        engine_state.add_env_var("PWD".into(), Value::test_string(cwd.to_string_lossy()));

        ide::complete(Arc::new(engine_state), &script_name, &ide_complete);

        return Ok(());
    } else if let Some(max_errors) = parsed_nu_cli_args.ide_check {
        ide::check(&mut engine_state, &script_name, &max_errors);

        return Ok(());
    } else if parsed_nu_cli_args.ide_ast.is_some() {
        ide::ast(&mut engine_state, &script_name);

        return Ok(());
    }

    start_time = std::time::Instant::now();
    if let Some(testbin) = &parsed_nu_cli_args.testbin {
        let dispatcher = test_bins::new_testbin_dispatcher();
        let test_bin = testbin.item.as_str();
        match dispatcher.get(test_bin) {
            Some(test_bin) => test_bin.run(),
            None => {
                if ["-h", "--help"].contains(&test_bin) {
                    test_bins::show_help(&dispatcher);
                } else {
                    eprintln!("ERROR: Unknown testbin '{test_bin}'");
                    std::process::exit(1);
                }
            }
        }
        std::process::exit(0)
    } else {
        // If we're not running a testbin, set the current working directory to
        // the location of the Nushell executable. This prevents the OS from
        // locking the directory where the user launched Nushell.
        std::env::set_current_dir(current_exe_directory())
            .expect("set_current_dir() should succeed");
    }
    perf!("run test_bins", start_time, use_color);

    start_time = std::time::Instant::now();
    let input = if let Some(redirect_stdin) = &parsed_nu_cli_args.redirect_stdin {
        trace!("redirecting stdin");
        PipelineData::byte_stream(ByteStream::stdin(redirect_stdin.span)?, None)
    } else {
        trace!("not redirecting stdin");
        PipelineData::empty()
    };
    perf!("redirect stdin", start_time, use_color);

    start_time = std::time::Instant::now();
    // Set up the $nu constant before evaluating config files (need to have $nu available in them)
    engine_state.generate_nu_constant();
    perf!("create_nu_constant", start_time, use_color);

    #[cfg(feature = "plugin")]
    if let Some(plugins) = &parsed_nu_cli_args.plugins {
        use nu_plugin_engine::{GetPlugin, PluginDeclaration};
        use nu_protocol::{ErrSpan, PluginIdentity, RegisteredPlugin, engine::StateWorkingSet};

        // Load any plugins specified with --plugins
        start_time = std::time::Instant::now();

        let mut working_set = StateWorkingSet::new(&engine_state);
        for plugin_filename in plugins {
            // Make sure the plugin filenames are canonicalized
            let filename = canonicalize_with(&plugin_filename.item, &init_cwd)
                .map_err(|err| {
                    nu_protocol::shell_error::io::IoError::new(
                        err,
                        plugin_filename.span,
                        PathBuf::from(&plugin_filename.item),
                    )
                })
                .map_err(ShellError::from)?;

            let identity = PluginIdentity::new(&filename, None)
                .err_span(plugin_filename.span)
                .map_err(ShellError::from)?;

            // Create the plugin and add it to the working set
            let plugin = nu_plugin_engine::add_plugin_to_working_set(&mut working_set, &identity)?;

            // Spawn the plugin to get the metadata and signatures
            let interface = plugin.clone().get_plugin(None)?;

            // Set its metadata
            plugin.set_metadata(Some(interface.get_metadata()?));

            // Add the commands from the signature to the working set
            for signature in interface.get_signature()? {
                let decl = PluginDeclaration::new(plugin.clone(), signature);
                working_set.add_decl(Box::new(decl));
            }
        }
        engine_state.merge_delta(working_set.render())?;

        perf!("load plugins specified in --plugins", start_time, use_color)
    }

    start_time = std::time::Instant::now();
    if parsed_nu_cli_args.lsp {
        perf!("lsp starting", start_time, use_color);

        if parsed_nu_cli_args.no_config_file.is_none() {
            let mut stack = nu_protocol::engine::Stack::new();
            config_files::setup_config(
                &mut engine_state,
                &mut stack,
                #[cfg(feature = "plugin")]
                parsed_nu_cli_args.plugin_file,
                parsed_nu_cli_args.config_file,
                parsed_nu_cli_args.env_file,
                false,
            );
        }

        LanguageServer::initialize_stdio_connection(engine_state)?.serve_requests()?
    } else if let Some(commands) = parsed_nu_cli_args.commands.clone() {
        run_commands(
            &mut engine_state,
            stack,
            parsed_nu_cli_args,
            use_color,
            &commands,
            input,
            entire_start_time,
        );

        cleanup_exit(0, &engine_state, 0);
    } else if !script_name.is_empty() {
        run_file(
            &mut engine_state,
            stack,
            parsed_nu_cli_args,
            use_color,
            script_name,
            args_to_script,
            input,
        );

        cleanup_exit(0, &engine_state, 0);
    } else {
        // Environment variables that apply only when in REPL
        engine_state.add_env_var("PROMPT_INDICATOR".to_string(), Value::test_string("> "));
        engine_state.add_env_var(
            "PROMPT_INDICATOR_VI_NORMAL".to_string(),
            Value::test_string("> "),
        );
        engine_state.add_env_var(
            "PROMPT_INDICATOR_VI_INSERT".to_string(),
            Value::test_string(": "),
        );
        engine_state.add_env_var(
            "PROMPT_MULTILINE_INDICATOR".to_string(),
            Value::test_string("::: "),
        );
        engine_state.add_env_var(
            "TRANSIENT_PROMPT_MULTILINE_INDICATOR".to_string(),
            Value::test_string(""),
        );
        engine_state.add_env_var(
            "TRANSIENT_PROMPT_COMMAND_RIGHT".to_string(),
            Value::test_string(""),
        );
        let mut shlvl = engine_state
            .get_env_var("SHLVL")
            .map(|x| x.as_str().unwrap_or("0").parse::<i64>().unwrap_or(0))
            .unwrap_or(0);
        shlvl += 1;
        engine_state.add_env_var("SHLVL".to_string(), Value::int(shlvl, Span::unknown()));

        run_repl(
            &mut engine_state,
            stack,
            parsed_nu_cli_args,
            entire_start_time,
        )?;

        cleanup_exit(0, &engine_state, 0);
    }

    Ok(())
}
