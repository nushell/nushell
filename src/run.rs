use crate::{
    command,
    config_files::{self, setup_config},
};
use log::trace;
#[cfg(feature = "plugin")]
use nu_cli::read_plugin_file;
use nu_cli::{EvaluateCommandsOpts, evaluate_commands, evaluate_file, evaluate_repl};
use nu_protocol::{
    PipelineData, Spanned,
    engine::{EngineState, Stack},
    report_shell_error,
};
use nu_utils::perf;

pub(crate) fn run_commands(
    engine_state: &mut EngineState,
    mut stack: Stack,
    parsed_nu_cli_args: command::NushellCliArgs,
    use_color: bool,
    commands: &Spanned<String>,
    input: PipelineData,
    entire_start_time: std::time::Instant,
) {
    trace!("run_commands");

    let start_time = std::time::Instant::now();
    let create_scaffold = nu_path::nu_config_dir().is_some_and(|p| !p.exists());

    // if the --no-config-file(-n) option is NOT passed, load the plugin file,
    // load the default env file or custom (depending on parsed_nu_cli_args.env_file),
    // and maybe a custom config file (depending on parsed_nu_cli_args.config_file)
    //
    // if the --no-config-file(-n) flag is passed, do not load plugin, env, or config files
    if parsed_nu_cli_args.no_config_file.is_none() {
        #[cfg(feature = "plugin")]
        read_plugin_file(engine_state, parsed_nu_cli_args.plugin_file);

        perf!("read plugins", start_time, use_color);

        let start_time = std::time::Instant::now();
        // If we have a env file parameter *OR* we have a login shell parameter, read the env file
        if parsed_nu_cli_args.env_file.is_some() || parsed_nu_cli_args.login_shell.is_some() {
            config_files::read_config_file(
                engine_state,
                &mut stack,
                parsed_nu_cli_args.env_file,
                nu_utils::ConfigFileKind::Env,
                create_scaffold,
            );
        } else {
            config_files::read_default_env_file(engine_state, &mut stack)
        }

        perf!("read env.nu", start_time, use_color);

        let start_time = std::time::Instant::now();
        let create_scaffold = nu_path::nu_config_dir().is_some_and(|p| !p.exists());

        // If we have a config file parameter *OR* we have a login shell parameter, read the config file
        if parsed_nu_cli_args.config_file.is_some() || parsed_nu_cli_args.login_shell.is_some() {
            config_files::read_config_file(
                engine_state,
                &mut stack,
                parsed_nu_cli_args.config_file,
                nu_utils::ConfigFileKind::Config,
                create_scaffold,
            );
        }

        perf!("read config.nu", start_time, use_color);

        // If we have a login shell parameter, read the login file
        let start_time = std::time::Instant::now();
        if parsed_nu_cli_args.login_shell.is_some() {
            config_files::read_loginshell_file(engine_state, &mut stack);
        }

        perf!("read login.nu", start_time, use_color);
    }

    // Before running commands, set up the startup time
    engine_state.set_startup_time(entire_start_time.elapsed().as_nanos() as i64);

    // Regenerate the $nu constant to contain the startup time and any other potential updates
    engine_state.generate_nu_constant();

    let start_time = std::time::Instant::now();
    let result = evaluate_commands(
        commands,
        engine_state,
        &mut stack,
        input,
        EvaluateCommandsOpts {
            table_mode: parsed_nu_cli_args.table_mode,
            error_style: parsed_nu_cli_args.error_style,
            no_newline: parsed_nu_cli_args.no_newline.is_some(),
        },
    );
    perf!("evaluate_commands", start_time, use_color);

    if let Err(err) = result {
        report_shell_error(engine_state, &err);
        std::process::exit(err.exit_code().unwrap_or(0));
    }
}

pub(crate) fn run_file(
    engine_state: &mut EngineState,
    mut stack: Stack,
    parsed_nu_cli_args: command::NushellCliArgs,
    use_color: bool,
    script_name: String,
    args_to_script: Vec<String>,
    input: PipelineData,
) {
    trace!("run_file");

    // if the --no-config-file(-n) option is NOT passed, load the plugin file,
    // load the default env file or custom (depending on parsed_nu_cli_args.env_file),
    // and maybe a custom config file (depending on parsed_nu_cli_args.config_file)
    //
    // if the --no-config-file(-n) flag is passed, do not load plugin, env, or config files
    if parsed_nu_cli_args.no_config_file.is_none() {
        let start_time = std::time::Instant::now();
        let create_scaffold = nu_path::nu_config_dir().is_some_and(|p| !p.exists());
        #[cfg(feature = "plugin")]
        read_plugin_file(engine_state, parsed_nu_cli_args.plugin_file);
        perf!("read plugins", start_time, use_color);

        let start_time = std::time::Instant::now();
        // only want to load config and env if relative argument is provided.
        if parsed_nu_cli_args.env_file.is_some() {
            config_files::read_config_file(
                engine_state,
                &mut stack,
                parsed_nu_cli_args.env_file,
                nu_utils::ConfigFileKind::Env,
                create_scaffold,
            );
        } else {
            config_files::read_default_env_file(engine_state, &mut stack)
        }
        perf!("read env.nu", start_time, use_color);

        let start_time = std::time::Instant::now();
        if parsed_nu_cli_args.config_file.is_some() {
            config_files::read_config_file(
                engine_state,
                &mut stack,
                parsed_nu_cli_args.config_file,
                nu_utils::ConfigFileKind::Config,
                create_scaffold,
            );
        }
        perf!("read config.nu", start_time, use_color);
    }

    // Regenerate the $nu constant to contain the startup time and any other potential updates
    engine_state.generate_nu_constant();

    let start_time = std::time::Instant::now();
    let result = evaluate_file(
        script_name,
        &args_to_script,
        engine_state,
        &mut stack,
        input,
    );
    perf!("evaluate_file", start_time, use_color);

    if let Err(err) = result {
        report_shell_error(engine_state, &err);
        std::process::exit(err.exit_code().unwrap_or(0));
    }
}

pub(crate) fn run_repl(
    engine_state: &mut EngineState,
    mut stack: Stack,
    parsed_nu_cli_args: command::NushellCliArgs,
    entire_start_time: std::time::Instant,
) -> Result<(), miette::ErrReport> {
    trace!("run_repl");
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
    let use_color = engine_state
        .get_config()
        .use_ansi_coloring
        .get(engine_state);
    perf!("setup_config", start_time, use_color);

    let start_time = std::time::Instant::now();
    let ret_val = evaluate_repl(
        engine_state,
        stack,
        parsed_nu_cli_args.execute,
        parsed_nu_cli_args.no_std_lib,
        entire_start_time,
    );
    perf!("evaluate_repl", start_time, use_color);

    ret_val
}
