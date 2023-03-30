#[cfg(feature = "plugin")]
use crate::config_files::NUSHELL_FOLDER;
use crate::{
    command,
    config_files::{self, setup_config},
};
#[cfg(feature = "plugin")]
use nu_cli::read_plugin_file;
use nu_cli::{evaluate_commands, evaluate_file, evaluate_repl, report_error};
use nu_parser::parse_module_block;
use nu_protocol::{engine::StateWorkingSet, PipelineData, ShellError, Span};
use nu_utils::utils::perf;

pub(crate) fn run_commands(
    engine_state: &mut nu_protocol::engine::EngineState,
    parsed_nu_cli_args: command::NushellCliArgs,
    use_color: bool,
    commands: &nu_protocol::Spanned<String>,
    input: PipelineData,
    entire_start_time: std::time::Instant,
) -> Result<(), miette::ErrReport> {
    let mut stack = nu_protocol::engine::Stack::new();
    let start_time = std::time::Instant::now();
    #[cfg(feature = "plugin")]
    read_plugin_file(
        engine_state,
        &mut stack,
        parsed_nu_cli_args.plugin_file,
        NUSHELL_FOLDER,
    );
    perf(
        "read plugins",
        start_time,
        file!(),
        line!(),
        column!(),
        use_color,
    );

    let start_time = std::time::Instant::now();
    // only want to load config and env if relative argument is provided.
    if parsed_nu_cli_args.env_file.is_some() {
        config_files::read_config_file(engine_state, &mut stack, parsed_nu_cli_args.env_file, true);
    } else {
        config_files::read_default_env_file(engine_state, &mut stack)
    }
    perf(
        "read env.nu",
        start_time,
        file!(),
        line!(),
        column!(),
        use_color,
    );

    let start_time = std::time::Instant::now();
    if parsed_nu_cli_args.config_file.is_some() {
        config_files::read_config_file(
            engine_state,
            &mut stack,
            parsed_nu_cli_args.config_file,
            false,
        );
    }
    perf(
        "read config.nu",
        start_time,
        file!(),
        line!(),
        column!(),
        use_color,
    );

    // Before running commands, set up the startup time
    engine_state.set_startup_time(entire_start_time.elapsed().as_nanos() as i64);
    let start_time = std::time::Instant::now();
    let ret_val = evaluate_commands(
        commands,
        engine_state,
        &mut stack,
        input,
        parsed_nu_cli_args.table_mode,
    );
    perf(
        "evaluate_commands",
        start_time,
        file!(),
        line!(),
        column!(),
        use_color,
    );

    match ret_val {
        Ok(Some(exit_code)) => std::process::exit(exit_code as i32),
        Ok(None) => Ok(()),
        Err(e) => Err(e),
    }
}

pub(crate) fn run_file(
    engine_state: &mut nu_protocol::engine::EngineState,
    parsed_nu_cli_args: command::NushellCliArgs,
    use_color: bool,
    script_name: String,
    args_to_script: Vec<String>,
    input: PipelineData,
) -> Result<(), miette::ErrReport> {
    let mut stack = nu_protocol::engine::Stack::new();
    let start_time = std::time::Instant::now();

    #[cfg(feature = "plugin")]
    read_plugin_file(
        engine_state,
        &mut stack,
        parsed_nu_cli_args.plugin_file,
        NUSHELL_FOLDER,
    );
    perf(
        "read plugins",
        start_time,
        file!(),
        line!(),
        column!(),
        use_color,
    );

    let start_time = std::time::Instant::now();
    // only want to load config and env if relative argument is provided.
    if parsed_nu_cli_args.env_file.is_some() {
        config_files::read_config_file(engine_state, &mut stack, parsed_nu_cli_args.env_file, true);
    } else {
        config_files::read_default_env_file(engine_state, &mut stack)
    }
    perf(
        "read env.nu",
        start_time,
        file!(),
        line!(),
        column!(),
        use_color,
    );

    let start_time = std::time::Instant::now();
    if parsed_nu_cli_args.config_file.is_some() {
        config_files::read_config_file(
            engine_state,
            &mut stack,
            parsed_nu_cli_args.config_file,
            false,
        );
    }
    perf(
        "read config.nu",
        start_time,
        file!(),
        line!(),
        column!(),
        use_color,
    );

    let start_time = std::time::Instant::now();
    let ret_val = evaluate_file(
        script_name,
        &args_to_script,
        engine_state,
        &mut stack,
        input,
    );
    perf(
        "evaluate_file",
        start_time,
        file!(),
        line!(),
        column!(),
        use_color,
    );

    let start_time = std::time::Instant::now();
    let last_exit_code = stack.get_env_var(&*engine_state, "LAST_EXIT_CODE");
    if let Some(last_exit_code) = last_exit_code {
        let value = last_exit_code.as_integer();
        if let Ok(value) = value {
            if value != 0 {
                std::process::exit(value as i32);
            }
        }
    }
    perf(
        "get exit code",
        start_time,
        file!(),
        line!(),
        column!(),
        use_color,
    );

    ret_val
}

fn get_standard_library() -> &'static str {
    include_str!("../crates/nu-utils/standard_library/std.nu")
}

pub(crate) fn run_repl(
    engine_state: &mut nu_protocol::engine::EngineState,
    parsed_nu_cli_args: command::NushellCliArgs,
    entire_start_time: std::time::Instant,
) -> Result<(), miette::ErrReport> {
    let mut stack = nu_protocol::engine::Stack::new();
    let start_time = std::time::Instant::now();

    if parsed_nu_cli_args.no_config_file.is_none() {
        setup_config(
            engine_state,
            &mut stack,
            #[cfg(feature = "plugin")]
            parsed_nu_cli_args.plugin_file,
            parsed_nu_cli_args.config_file,
            parsed_nu_cli_args.env_file,
            parsed_nu_cli_args.login_shell.is_some(),
        );
    }

    // Reload use_color from config in case it's different from the default value
    let use_color = engine_state.get_config().use_ansi_coloring;
    perf(
        "setup_config",
        start_time,
        file!(),
        line!(),
        column!(),
        use_color,
    );

    let delta = {
        let name = "std".to_string();
        let content = get_standard_library().as_bytes();

        let mut working_set = StateWorkingSet::new(engine_state);

        let start = working_set.next_span_start();
        working_set.add_file(name.clone(), content);
        let end = working_set.next_span_start();

        let (block, module, comments, _) = parse_module_block(
            &mut working_set,
            Span::new(start, end),
            name.as_bytes(),
            &[],
        );

        // TODO: change this when #8505 is merged
        // NOTE: remove the assert and uncomment the `help`s
        let prelude = vec![
            ("assert", "assert"),
            // ("help", "help"),
            // ("help commands", "help commands"),
            // ("help aliases", "help aliases"),
            // ("help modules", "help modules"),
            // ("help externs", "help externs"),
            // ("help operators", "help operators"),
        ];

        let mut decls = Vec::new();
        let mut errs = Vec::new();
        for (name, search_name) in prelude {
            if let Some(id) = module.decls.get(&search_name.as_bytes().to_vec()) {
                let decl = (name.as_bytes().to_vec(), id.to_owned());
                decls.push(decl);
            } else {
                errs.push(ShellError::GenericError(
                    format!("could not load `{}` from `std`.", search_name),
                    String::new(),
                    None,
                    None,
                    Vec::new(),
                ));
            }
        }

        if !errs.is_empty() {
            report_error(
                &working_set,
                &ShellError::GenericError(
                    "Unable to load the prelude of the standard library.".into(),
                    String::new(),
                    None,
                    Some("this is a bug: please file an issue at <issue_tracker_url>".to_string()),
                    errs,
                ),
            );
        }

        working_set.use_decls(decls);

        working_set.add_module(&name, module, comments);
        working_set.add_block(block);

        working_set.render()
    };

    engine_state.merge_delta(delta)?;

    let start_time = std::time::Instant::now();
    let ret_val = evaluate_repl(
        engine_state,
        &mut stack,
        config_files::NUSHELL_FOLDER,
        parsed_nu_cli_args.execute,
        entire_start_time,
    );
    perf(
        "evaluate_repl",
        start_time,
        file!(),
        line!(),
        column!(),
        use_color,
    );

    ret_val
}
