mod command;
mod config_files;
mod logger;
mod signals;
mod terminal;
mod test_bins;
#[cfg(test)]
mod tests;

#[cfg(feature = "plugin")]
use crate::config_files::NUSHELL_FOLDER;
use crate::{
    command::parse_commandline_args,
    config_files::{set_config_path, setup_config},
    logger::{configure, logger},
    terminal::acquire_terminal,
};
use devtimer::DevTime;
use log::{info, Level};
use miette::Result;
#[cfg(feature = "plugin")]
use nu_cli::read_plugin_file;
use nu_cli::{
    evaluate_commands, evaluate_file, evaluate_repl, gather_parent_env_vars, get_init_cwd,
    report_error_new,
};
use nu_command::create_default_context;
use nu_parser::{escape_for_script_arg, escape_quote_string};
use nu_protocol::{util::BufferedReader, PipelineData, RawStream};
use signals::{ctrlc_protection, sigquit_protection};
use std::str::FromStr;
use std::{
    io::BufReader,
    sync::{atomic::AtomicBool, Arc},
};

fn main() -> Result<()> {
    let mut start_time = DevTime::new_simple();
    start_time.start();
    let miette_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |x| {
        crossterm::terminal::disable_raw_mode().expect("unable to disable raw mode");
        miette_hook(x);
    }));

    // Get initial current working directory.
    let init_cwd = get_init_cwd();
    let mut engine_state = create_default_context();

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
    sigquit_protection(&mut engine_state);

    let mut args_to_nushell = vec![];
    let mut script_name = String::new();
    let mut args_to_script = vec![];

    // Would be nice if we had a way to parse this. The first flags we see will be going to nushell
    // then it'll be the script name
    // then the args to the script
    let mut args = std::env::args();
    let argv0 = args.next();

    while let Some(arg) = args.next() {
        if !script_name.is_empty() {
            args_to_script.push(escape_for_script_arg(&arg));
        } else if arg.starts_with('-') {
            // Cool, it's a flag
            let flag_value = match arg.as_ref() {
                "--commands" | "-c" | "--table-mode" | "-m" | "-e" | "--execute" => {
                    args.next().map(|a| escape_quote_string(&a))
                }
                "--config" | "--env-config" => args.next().map(|a| escape_quote_string(&a)),
                #[cfg(feature = "plugin")]
                "--plugin-config" => args.next().map(|a| escape_quote_string(&a)),
                "--log-level" | "--log-target" | "--testbin" | "--threads" | "-t" => args.next(),
                _ => None,
            };

            args_to_nushell.push(arg);

            if let Some(flag_value) = flag_value {
                args_to_nushell.push(flag_value);
            }
        } else {
            // Our script file
            script_name = arg;
        }
    }

    args_to_nushell.insert(0, "nu".into());

    if let Some(argv0) = argv0 {
        if argv0.starts_with('-') {
            args_to_nushell.push("--login".into());
        }
    }

    let nushell_commandline_args = args_to_nushell.join(" ");

    let parsed_nu_cli_args = parse_commandline_args(&nushell_commandline_args, &mut engine_state)
        .unwrap_or_else(|_| std::process::exit(1));

    set_config_path(
        &mut engine_state,
        &init_cwd,
        "config.nu",
        "config-path",
        &parsed_nu_cli_args.config_file,
    );

    set_config_path(
        &mut engine_state,
        &init_cwd,
        "env.nu",
        "env-path",
        &parsed_nu_cli_args.env_file,
    );

    // keep this condition in sync with the branches below
    acquire_terminal(
        parsed_nu_cli_args.commands.is_none()
            && (script_name.is_empty() || parsed_nu_cli_args.interactive_shell.is_some()),
    );

    if let Some(t) = parsed_nu_cli_args.threads {
        // 0 means to let rayon decide how many threads to use
        let threads = t.as_i64().unwrap_or(0);
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads as usize)
            .build_global()
            .expect("error setting number of threads");
    }

    if let Some(level) = parsed_nu_cli_args.log_level.map(|level| level.item) {
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
            .map(|target| target.item)
            .unwrap_or_else(|| "stderr".to_string());

        logger(|builder| configure(&level, &target, builder))?;
        info!("start logging {}:{}:{}", file!(), line!(), column!());
    }

    if let Some(testbin) = &parsed_nu_cli_args.testbin {
        // Call out to the correct testbin
        match testbin.item.as_str() {
            "echo_env" => test_bins::echo_env(true),
            "echo_env_stderr" => test_bins::echo_env(false),
            "cococo" => test_bins::cococo(),
            "meow" => test_bins::meow(),
            "meowb" => test_bins::meowb(),
            "relay" => test_bins::relay(),
            "iecho" => test_bins::iecho(),
            "fail" => test_bins::fail(),
            "nonu" => test_bins::nonu(),
            "chop" => test_bins::chop(),
            "repeater" => test_bins::repeater(),
            "nu_repl" => test_bins::nu_repl(),
            _ => std::process::exit(1),
        }
        std::process::exit(0)
    }
    let input = if let Some(redirect_stdin) = &parsed_nu_cli_args.redirect_stdin {
        let stdin = std::io::stdin();
        let buf_reader = BufReader::new(stdin);

        PipelineData::ExternalStream {
            stdout: Some(RawStream::new(
                Box::new(BufferedReader::new(buf_reader)),
                Some(ctrlc),
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
        PipelineData::empty()
    };

    info!("redirect_stdin {}:{}:{}", file!(), line!(), column!());

    // First, set up env vars as strings only
    gather_parent_env_vars(&mut engine_state, &init_cwd);

    let mut stack = nu_protocol::engine::Stack::new();

    if let Some(commands) = &parsed_nu_cli_args.commands {
        #[cfg(feature = "plugin")]
        read_plugin_file(
            &mut engine_state,
            &mut stack,
            parsed_nu_cli_args.plugin_file,
            NUSHELL_FOLDER,
        );

        // only want to load config and env if relative argument is provided.
        if parsed_nu_cli_args.env_file.is_some() {
            config_files::read_config_file(
                &mut engine_state,
                &mut stack,
                parsed_nu_cli_args.env_file,
                true,
            );
        } else {
            config_files::read_default_env_file(&mut engine_state, &mut stack)
        }

        if parsed_nu_cli_args.config_file.is_some() {
            config_files::read_config_file(
                &mut engine_state,
                &mut stack,
                parsed_nu_cli_args.config_file,
                false,
            );
        }

        let ret_val = evaluate_commands(
            commands,
            &mut engine_state,
            &mut stack,
            input,
            parsed_nu_cli_args.table_mode,
        );
        info!("-c command execution {}:{}:{}", file!(), line!(), column!());
        match ret_val {
            Ok(Some(exit_code)) => std::process::exit(exit_code as i32),
            Ok(None) => Ok(()),
            Err(e) => Err(e),
        }
    } else if !script_name.is_empty() && parsed_nu_cli_args.interactive_shell.is_none() {
        #[cfg(feature = "plugin")]
        read_plugin_file(
            &mut engine_state,
            &mut stack,
            parsed_nu_cli_args.plugin_file,
            NUSHELL_FOLDER,
        );

        // only want to load config and env if relative argument is provided.
        if parsed_nu_cli_args.env_file.is_some() {
            config_files::read_config_file(
                &mut engine_state,
                &mut stack,
                parsed_nu_cli_args.env_file,
                true,
            );
        } else {
            config_files::read_default_env_file(&mut engine_state, &mut stack)
        }

        if parsed_nu_cli_args.config_file.is_some() {
            config_files::read_config_file(
                &mut engine_state,
                &mut stack,
                parsed_nu_cli_args.config_file,
                false,
            );
        }

        let ret_val = evaluate_file(
            script_name,
            &args_to_script,
            &mut engine_state,
            &mut stack,
            input,
        );

        let last_exit_code = stack.get_env_var(&engine_state, "LAST_EXIT_CODE");
        if let Some(last_exit_code) = last_exit_code {
            let value = last_exit_code.as_integer();
            if let Ok(value) = value {
                if value != 0 {
                    std::process::exit(value as i32);
                }
            }
        }
        info!("eval_file execution {}:{}:{}", file!(), line!(), column!());

        ret_val
    } else {
        setup_config(
            &mut engine_state,
            &mut stack,
            #[cfg(feature = "plugin")]
            parsed_nu_cli_args.plugin_file,
            parsed_nu_cli_args.config_file,
            parsed_nu_cli_args.env_file,
            parsed_nu_cli_args.login_shell.is_some(),
        );

        let ret_val = evaluate_repl(
            &mut engine_state,
            &mut stack,
            config_files::NUSHELL_FOLDER,
            parsed_nu_cli_args.execute,
            &mut start_time,
        );
        info!("repl eval {}:{}:{}", file!(), line!(), column!());

        ret_val
    }
}
