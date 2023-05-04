#[cfg(feature = "plugin")]
use crate::config_files::NUSHELL_FOLDER;
use crate::{
    command,
    config_files::{self, setup_config},
};
#[cfg(feature = "plugin")]
use nu_cli::read_plugin_file;
use nu_cli::{evaluate_commands, evaluate_file, evaluate_repl};
use nu_protocol::PipelineData;
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
