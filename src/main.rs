mod command;
mod config_files;
mod logger;
mod run;
mod signals;
mod terminal;
mod test_bins;
#[cfg(test)]
mod tests;

use crate::{
    command::parse_commandline_args,
    config_files::set_config_path,
    logger::{configure, logger},
    terminal::acquire_terminal,
};
use command::gather_commandline_args;
use log::Level;
use miette::Result;
use nu_cli::{gather_parent_env_vars, get_init_cwd, report_error_new};
use nu_command::create_default_context;
use nu_protocol::{util::BufferedReader, PipelineData, RawStream};
use nu_utils::utils::perf;
use run::{run_commands, run_file, run_repl};
use signals::{ctrlc_protection, sigquit_protection};
use std::{
    io::BufReader,
    str::FromStr,
    sync::{atomic::AtomicBool, Arc},
};

fn main() -> Result<()> {
    let entire_start_time = std::time::Instant::now();
    let mut start_time = std::time::Instant::now();
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

    let (args_to_nushell, script_name, args_to_script) = gather_commandline_args();
    let parsed_nu_cli_args = parse_commandline_args(&args_to_nushell.join(" "), &mut engine_state)
        .unwrap_or_else(|_| std::process::exit(1));

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
        &parsed_nu_cli_args.config_file,
    );

    set_config_path(
        &mut engine_state,
        &init_cwd,
        "env.nu",
        "env-path",
        &parsed_nu_cli_args.env_file,
    );
    perf(
        "set_config_path",
        start_time,
        file!(),
        line!(),
        column!(),
        use_color,
    );

    start_time = std::time::Instant::now();
    // keep this condition in sync with the branches below
    acquire_terminal(
        parsed_nu_cli_args.commands.is_none()
            && (script_name.is_empty() || parsed_nu_cli_args.interactive_shell.is_some()),
    );
    perf(
        "acquire_terminal",
        start_time,
        file!(),
        line!(),
        column!(),
        use_color,
    );

    start_time = std::time::Instant::now();
    if let Some(t) = parsed_nu_cli_args.threads.clone() {
        // 0 means to let rayon decide how many threads to use
        let threads = t.as_i64().unwrap_or(0);
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads as usize)
            .build_global()
            .expect("error setting number of threads");
    }
    perf(
        "set rayon threads",
        start_time,
        file!(),
        line!(),
        column!(),
        use_color,
    );

    start_time = std::time::Instant::now();
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
    perf(
        "redirect stdin",
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

    if let Some(commands) = parsed_nu_cli_args.commands.clone() {
        run_commands(
            &mut engine_state,
            parsed_nu_cli_args,
            use_color,
            &commands,
            input,
        )
    } else if !script_name.is_empty() && parsed_nu_cli_args.interactive_shell.is_none() {
        run_file(
            &mut engine_state,
            parsed_nu_cli_args,
            use_color,
            script_name,
            args_to_script,
            input,
        )
    } else {
        run_repl(engine_state, parsed_nu_cli_args, entire_start_time)
    }
}
