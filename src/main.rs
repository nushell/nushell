mod command;
mod config_files;
mod ide;
mod logger;
mod run;
mod signals;
#[cfg(unix)]
mod terminal;
mod test_bins;
#[cfg(test)]
mod tests;

#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use crate::{
    command::parse_commandline_args,
    config_files::set_config_path,
    logger::{configure, logger},
};
use command::gather_commandline_args;
use log::{trace, Level};
use miette::Result;
use nu_cli::gather_parent_env_vars;
use nu_cmd_base::util::get_init_cwd;
use nu_lsp::LanguageServer;
use nu_path::canonicalize_with;
use nu_protocol::{
    engine::EngineState, eval_const::create_nu_constant, report_error_new, util::BufferedReader,
    PipelineData, RawStream, ShellError, Span, Value, NU_VARIABLE_ID,
};
use nu_std::load_standard_library;
use nu_utils::utils::perf;
use run::{run_commands, run_file, run_repl};
use signals::ctrlc_protection;
use std::{
    io::BufReader,
    path::PathBuf,
    str::FromStr,
    sync::{atomic::AtomicBool, Arc},
};

fn get_engine_state() -> EngineState {
    let engine_state = nu_cmd_lang::create_default_context();
    #[cfg(feature = "plugin")]
    let engine_state = nu_cmd_plugin::add_plugin_command_context(engine_state);
    let engine_state = nu_command::add_shell_command_context(engine_state);
    let engine_state = nu_cmd_extra::add_extra_command_context(engine_state);
    #[cfg(feature = "dataframe")]
    let engine_state = nu_cmd_dataframe::add_dataframe_context(engine_state);
    let engine_state = nu_cli::add_cli_context(engine_state);
    nu_explore::add_explore_context(engine_state)
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

    // Get initial current working directory.
    let init_cwd = get_init_cwd();
    let mut engine_state = get_engine_state();

    // Custom additions
    let delta = {
        let mut working_set = nu_protocol::engine::StateWorkingSet::new(&engine_state);
        working_set.add_decl(Box::new(nu_cli::NuHighlight));
        working_set.add_decl(Box::new(nu_cli::Print));
        working_set.render()
    };

    if let Err(err) = engine_state.merge_delta(delta) {
        report_error_new(&engine_state, &err);
    }

    let ctrlc = Arc::new(AtomicBool::new(false));
    // TODO: make this conditional in the future
    ctrlc_protection(&mut engine_state, &ctrlc);

    // Begin: Default NU_LIB_DIRS, NU_PLUGIN_DIRS
    // Set default NU_LIB_DIRS and NU_PLUGIN_DIRS here before the env.nu is processed. If
    // the env.nu file exists, these values will be overwritten, if it does not exist, or
    // there is an error reading it, these values will be used.
    let nushell_config_path = if let Some(mut path) = nu_path::config_dir() {
        path.push("nushell");
        path
    } else {
        // Not really sure what to default this to if nu_path::config_dir() returns None
        std::path::PathBuf::new()
    };

    if let Ok(xdg_config_home) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg_config_home.is_empty() {
            if nushell_config_path
                != canonicalize_with(&xdg_config_home, &init_cwd)
                    .unwrap_or(PathBuf::from(&xdg_config_home))
                    .join("nushell")
            {
                report_error_new(
                    &engine_state,
                    &ShellError::InvalidXdgConfig {
                        xdg: xdg_config_home,
                        default: nushell_config_path.display().to_string(),
                    },
                );
            } else if let Some(old_config) = nu_path::config_dir_old().map(|p| p.join("nushell")) {
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
    }

    let mut default_nu_lib_dirs_path = nushell_config_path.clone();
    default_nu_lib_dirs_path.push("scripts");
    engine_state.add_env_var(
        "NU_LIB_DIRS".to_string(),
        Value::test_list(vec![Value::test_string(
            default_nu_lib_dirs_path.to_string_lossy(),
        )]),
    );

    let mut default_nu_plugin_dirs_path = nushell_config_path;
    default_nu_plugin_dirs_path.push("plugins");
    engine_state.add_env_var(
        "NU_PLUGIN_DIRS".to_string(),
        Value::test_list(vec![Value::test_string(
            default_nu_plugin_dirs_path.to_string_lossy(),
        )]),
    );
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

    let (args_to_nushell, script_name, args_to_script) = gather_commandline_args();
    let parsed_nu_cli_args = parse_commandline_args(&args_to_nushell.join(" "), &mut engine_state)
        .unwrap_or_else(|_| std::process::exit(1));

    // keep this condition in sync with the branches at the end
    engine_state.is_interactive = parsed_nu_cli_args.interactive_shell.is_some()
        || (parsed_nu_cli_args.testbin.is_none()
            && parsed_nu_cli_args.commands.is_none()
            && script_name.is_empty());

    engine_state.is_login = parsed_nu_cli_args.login_shell.is_some();

    engine_state.history_enabled = parsed_nu_cli_args.no_history.is_none();

    let use_color = engine_state.get_config().use_ansi_coloring;
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

        logger(|builder| configure(&level, &target, builder))?;
        // info!("start logging {}:{}:{}", file!(), line!(), column!());
        perf(
            "start logging",
            start_time,
            file!(),
            line!(),
            column!(),
            use_color,
        );
    }

    start_time = std::time::Instant::now();
    set_config_path(
        &mut engine_state,
        &init_cwd,
        "config.nu",
        "config-path",
        parsed_nu_cli_args.config_file.as_ref(),
    );

    set_config_path(
        &mut engine_state,
        &init_cwd,
        "env.nu",
        "env-path",
        parsed_nu_cli_args.env_file.as_ref(),
    );
    perf(
        "set_config_path",
        start_time,
        file!(),
        line!(),
        column!(),
        use_color,
    );

    #[cfg(unix)]
    {
        start_time = std::time::Instant::now();
        terminal::acquire(engine_state.is_interactive);
        perf(
            "acquire_terminal",
            start_time,
            file!(),
            line!(),
            column!(),
            use_color,
        );
    }

    start_time = std::time::Instant::now();
    if let Some(include_path) = &parsed_nu_cli_args.include_path {
        let span = include_path.span;
        let vals: Vec<_> = include_path
            .item
            .split('\x1e') // \x1e is the record separator character (a character that is unlikely to appear in a path)
            .map(|x| Value::string(x.trim().to_string(), span))
            .collect();

        engine_state.add_env_var("NU_LIB_DIRS".into(), Value::list(vals, span));
    }
    perf(
        "NU_LIB_DIRS setup",
        start_time,
        file!(),
        line!(),
        column!(),
        use_color,
    );

    start_time = std::time::Instant::now();
    // First, set up env vars as strings only
    gather_parent_env_vars(&mut engine_state, &init_cwd);
    perf(
        "gather env vars",
        start_time,
        file!(),
        line!(),
        column!(),
        use_color,
    );

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
        // Call out to the correct testbin
        match testbin.item.as_str() {
            "echo_env" => test_bins::echo_env(true),
            "echo_env_stderr" => test_bins::echo_env(false),
            "echo_env_stderr_fail" => test_bins::echo_env_and_fail(false),
            "echo_env_mixed" => test_bins::echo_env_mixed(),
            "cococo" => test_bins::cococo(),
            "meow" => test_bins::meow(),
            "meowb" => test_bins::meowb(),
            "relay" => test_bins::relay(),
            "iecho" => test_bins::iecho(),
            "fail" => test_bins::fail(),
            "nonu" => test_bins::nonu(),
            "chop" => test_bins::chop(),
            "repeater" => test_bins::repeater(),
            "repeat_bytes" => test_bins::repeat_bytes(),
            "nu_repl" => test_bins::nu_repl(),
            "input_bytes_length" => test_bins::input_bytes_length(),
            _ => std::process::exit(1),
        }
        std::process::exit(0)
    }
    perf(
        "run test_bins",
        start_time,
        file!(),
        line!(),
        column!(),
        use_color,
    );

    start_time = std::time::Instant::now();
    let input = if let Some(redirect_stdin) = &parsed_nu_cli_args.redirect_stdin {
        trace!("redirecting stdin");
        let stdin = std::io::stdin();
        let buf_reader = BufReader::new(stdin);

        PipelineData::ExternalStream {
            stdout: Some(RawStream::new(
                Box::new(BufferedReader::new(buf_reader)),
                Some(ctrlc.clone()),
                redirect_stdin.span,
                None,
            )),
            stderr: None,
            exit_code: None,
            span: redirect_stdin.span,
            metadata: None,
            trim_end_newline: false,
        }
    } else {
        trace!("not redirecting stdin");
        PipelineData::empty()
    };
    perf(
        "redirect stdin",
        start_time,
        file!(),
        line!(),
        column!(),
        use_color,
    );

    start_time = std::time::Instant::now();
    // Set up the $nu constant before evaluating config files (need to have $nu available in them)
    let nu_const = create_nu_constant(&engine_state, input.span().unwrap_or_else(Span::unknown))?;
    engine_state.set_variable_const_val(NU_VARIABLE_ID, nu_const);
    perf(
        "create_nu_constant",
        start_time,
        file!(),
        line!(),
        column!(),
        use_color,
    );

    #[cfg(feature = "plugin")]
    if let Some(plugins) = &parsed_nu_cli_args.plugins {
        use nu_plugin::{GetPlugin, PluginDeclaration};
        use nu_protocol::{engine::StateWorkingSet, ErrSpan, PluginIdentity};

        // Load any plugins specified with --plugins
        start_time = std::time::Instant::now();

        let mut working_set = StateWorkingSet::new(&engine_state);
        for plugin_filename in plugins {
            // Make sure the plugin filenames are canonicalized
            let filename = canonicalize_with(&plugin_filename.item, &init_cwd)
                .err_span(plugin_filename.span)
                .map_err(ShellError::from)?;

            let identity = PluginIdentity::new(&filename, None)
                .err_span(plugin_filename.span)
                .map_err(ShellError::from)?;

            // Create the plugin and add it to the working set
            let plugin = nu_plugin::add_plugin_to_working_set(&mut working_set, &identity)?;

            // Spawn the plugin to get its signatures, and then add the commands to the working set
            for signature in plugin.clone().get_plugin(None)?.get_signature()? {
                let decl = PluginDeclaration::new(plugin.clone(), signature);
                working_set.add_decl(Box::new(decl));
            }
        }
        engine_state.merge_delta(working_set.render())?;

        perf(
            "load plugins specified in --plugins",
            start_time,
            file!(),
            line!(),
            column!(),
            use_color,
        )
    }

    start_time = std::time::Instant::now();
    if parsed_nu_cli_args.lsp {
        perf(
            "lsp starting",
            start_time,
            file!(),
            line!(),
            column!(),
            use_color,
        );

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

        LanguageServer::initialize_stdio_connection()?.serve_requests(engine_state, ctrlc)
    } else if let Some(commands) = parsed_nu_cli_args.commands.clone() {
        run_commands(
            &mut engine_state,
            parsed_nu_cli_args,
            use_color,
            &commands,
            input,
            entire_start_time,
        )
    } else if !script_name.is_empty() {
        run_file(
            &mut engine_state,
            parsed_nu_cli_args,
            use_color,
            script_name,
            args_to_script,
            input,
        )
    } else {
        run_repl(&mut engine_state, parsed_nu_cli_args, entire_start_time)
    }
}
