mod config_files;
mod logger;
mod test_bins;
#[cfg(test)]
mod tests;

#[cfg(feature = "plugin")]
use crate::config_files::NUSHELL_FOLDER;
use crate::logger::{configure, logger};
use log::{info, Level};
use miette::Result;
#[cfg(feature = "plugin")]
use nu_cli::read_plugin_file;
use nu_cli::{
    evaluate_commands, evaluate_file, evaluate_repl, gather_parent_env_vars, get_init_cwd,
    report_error, report_error_new,
};
use nu_command::{create_default_context, BufferedReader};
use nu_engine::{get_full_help, CallExt};
use nu_parser::{escape_for_script_arg, escape_quote_string, parse};
use nu_path::canonicalize_with;
use nu_protocol::{
    ast::{Call, Expr, Expression},
    engine::{Command, EngineState, Stack, StateWorkingSet},
    Category, Example, IntoPipelineData, PipelineData, RawStream, ShellError, Signature, Span,
    Spanned, SyntaxShape, Value,
};
use nu_utils::stdout_write_all_and_flush;
use std::{
    io::BufReader,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use std::{path::Path, str::FromStr};

// Inspired by fish's acquire_tty_or_exit
#[cfg(unix)]
fn take_control(interactive: bool) {
    use nix::{
        errno::Errno,
        sys::signal::{self, SaFlags, SigAction, SigHandler, SigSet, Signal},
        unistd::{self, Pid},
    };

    let shell_pgid = unistd::getpgrp();

    match unistd::tcgetpgrp(nix::libc::STDIN_FILENO) {
        Ok(owner_pgid) if owner_pgid == shell_pgid => {
            // Common case, nothing to do
            return;
        }
        Ok(owner_pgid) if owner_pgid == unistd::getpid() => {
            // This can apparently happen with sudo: https://github.com/fish-shell/fish-shell/issues/7388
            let _ = unistd::setpgid(owner_pgid, owner_pgid);
            return;
        }
        _ => (),
    }

    // Reset all signal handlers to default
    for sig in Signal::iterator() {
        unsafe {
            if let Ok(old_act) = signal::sigaction(
                sig,
                &SigAction::new(SigHandler::SigDfl, SaFlags::empty(), SigSet::empty()),
            ) {
                // fish preserves ignored SIGHUP, presumably for nohup support, so let's do the same
                if sig == Signal::SIGHUP && old_act.handler() == SigHandler::SigIgn {
                    let _ = signal::sigaction(sig, &old_act);
                }
            }
        }
    }

    let mut success = false;
    for _ in 0..4096 {
        match unistd::tcgetpgrp(nix::libc::STDIN_FILENO) {
            Ok(owner_pgid) if owner_pgid == shell_pgid => {
                success = true;
                break;
            }
            Ok(owner_pgid) if owner_pgid == Pid::from_raw(0) => {
                // Zero basically means something like "not owned" and we can just take it
                let _ = unistd::tcsetpgrp(nix::libc::STDIN_FILENO, shell_pgid);
            }
            Err(Errno::ENOTTY) => {
                if !interactive {
                    // that's fine
                    return;
                }
                eprintln!("ERROR: no TTY for interactive shell");
                std::process::exit(1);
            }
            _ => {
                // fish also has other heuristics than "too many attempts" for the orphan check, but they're optional
                if signal::killpg(Pid::from_raw(-shell_pgid.as_raw()), Signal::SIGTTIN).is_err() {
                    if !interactive {
                        // that's fine
                        return;
                    }
                    eprintln!("ERROR: failed to SIGTTIN ourselves");
                    std::process::exit(1);
                }
            }
        }
    }
    if !success && interactive {
        eprintln!("ERROR: failed take control of the terminal, we might be orphaned");
        std::process::exit(1);
    }
}

#[cfg(unix)]
fn acquire_terminal(interactive: bool) {
    use nix::sys::signal::{signal, SigHandler, Signal};

    if !atty::is(atty::Stream::Stdin) {
        return;
    }

    take_control(interactive);

    unsafe {
        // SIGINT and SIGQUIT have special handling above
        signal(Signal::SIGTSTP, SigHandler::SigIgn).expect("signal ignore");
        signal(Signal::SIGTTIN, SigHandler::SigIgn).expect("signal ignore");
        signal(Signal::SIGTTOU, SigHandler::SigIgn).expect("signal ignore");
        // signal::signal(Signal::SIGCHLD, SigHandler::SigIgn).expect("signal ignore"); // needed for std::command's waitpid usage
    }
}

#[cfg(not(unix))]
fn acquire_terminal(_: bool) {}

fn main() -> Result<()> {
    // miette::set_panic_hook();
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

    // TODO: make this conditional in the future
    // Ctrl-c protection section
    let ctrlc = Arc::new(AtomicBool::new(false));
    let handler_ctrlc = ctrlc.clone();
    let engine_state_ctrlc = ctrlc.clone();

    ctrlc::set_handler(move || {
        handler_ctrlc.store(true, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    engine_state.ctrlc = Some(engine_state_ctrlc);
    // End ctrl-c protection section

    // SIGQUIT protection section (only works for POSIX system)
    #[cfg(not(windows))]
    {
        use signal_hook::consts::SIGQUIT;
        let sig_quit = Arc::new(AtomicBool::new(false));
        signal_hook::flag::register(SIGQUIT, sig_quit.clone()).expect("Error setting SIGQUIT flag");
        engine_state.set_sig_quit(sig_quit);
    }
    // End SIGQUIT protection section

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

    let parsed_nu_cli_args = parse_commandline_args(&nushell_commandline_args, &mut engine_state);

    if let Ok(ref args) = parsed_nu_cli_args {
        set_config_path(
            &mut engine_state,
            &init_cwd,
            "config.nu",
            "config-path",
            &args.config_file,
        );

        set_config_path(
            &mut engine_state,
            &init_cwd,
            "env.nu",
            "env-path",
            &args.env_file,
        );
    }

    match parsed_nu_cli_args {
        Ok(binary_args) => {
            // keep this condition in sync with the branches below
            acquire_terminal(
                binary_args.commands.is_none()
                    && (script_name.is_empty() || binary_args.interactive_shell.is_some()),
            );

            if let Some(t) = binary_args.threads {
                // 0 means to let rayon decide how many threads to use
                let threads = t.as_i64().unwrap_or(0);
                rayon::ThreadPoolBuilder::new()
                    .num_threads(threads as usize)
                    .build_global()
                    .expect("error setting number of threads");
            }

            if binary_args.log_level.is_some() {
                let mut level = binary_args
                    .log_level
                    .map(|level| level.item)
                    .unwrap_or_else(|| "info".to_string());

                if Level::from_str(level.as_str()).is_err() {
                    eprintln!("ERROR: log library did not recognize log level '{level}', using default 'info'");
                    level = "info".to_string();
                }

                let target = binary_args
                    .log_target
                    .map(|target| target.item)
                    .unwrap_or_else(|| "stderr".to_string());

                logger(|builder| configure(level.as_str(), target.as_str(), builder))?;
                info!("start logging {}:{}:{}", file!(), line!(), column!());
            }

            if let Some(testbin) = &binary_args.testbin {
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
            let input = if let Some(redirect_stdin) = &binary_args.redirect_stdin {
                let stdin = std::io::stdin();
                let buf_reader = BufReader::new(stdin);

                PipelineData::ExternalStream {
                    stdout: Some(RawStream::new(
                        Box::new(BufferedReader::new(buf_reader)),
                        Some(ctrlc),
                        redirect_stdin.span,
                    )),
                    stderr: None,
                    exit_code: None,
                    span: redirect_stdin.span,
                    metadata: None,
                }
            } else {
                PipelineData::new(Span::new(0, 0))
            };

            info!("redirect_stdin {}:{}:{}", file!(), line!(), column!());

            // First, set up env vars as strings only
            gather_parent_env_vars(&mut engine_state, &init_cwd);

            let mut stack = nu_protocol::engine::Stack::new();

            if let Some(commands) = &binary_args.commands {
                #[cfg(feature = "plugin")]
                read_plugin_file(
                    &mut engine_state,
                    &mut stack,
                    binary_args.plugin_file,
                    NUSHELL_FOLDER,
                );

                // only want to load config and env if relative argument is provided.
                if binary_args.env_file.is_some() {
                    config_files::read_config_file(
                        &mut engine_state,
                        &mut stack,
                        binary_args.env_file,
                        true,
                    );
                } else {
                    config_files::read_default_env_file(&mut engine_state, &mut stack)
                }

                if binary_args.config_file.is_some() {
                    config_files::read_config_file(
                        &mut engine_state,
                        &mut stack,
                        binary_args.config_file,
                        false,
                    );
                }

                let ret_val = evaluate_commands(
                    commands,
                    &mut engine_state,
                    &mut stack,
                    input,
                    binary_args.table_mode,
                );
                info!("-c command execution {}:{}:{}", file!(), line!(), column!());
                match ret_val {
                    Ok(Some(exit_code)) => std::process::exit(exit_code as i32),
                    Ok(None) => Ok(()),
                    Err(e) => Err(e),
                }
            } else if !script_name.is_empty() && binary_args.interactive_shell.is_none() {
                #[cfg(feature = "plugin")]
                read_plugin_file(
                    &mut engine_state,
                    &mut stack,
                    binary_args.plugin_file,
                    NUSHELL_FOLDER,
                );

                // only want to load config and env if relative argument is provided.
                if binary_args.env_file.is_some() {
                    config_files::read_config_file(
                        &mut engine_state,
                        &mut stack,
                        binary_args.env_file,
                        true,
                    );
                } else {
                    config_files::read_default_env_file(&mut engine_state, &mut stack)
                }

                if binary_args.config_file.is_some() {
                    config_files::read_config_file(
                        &mut engine_state,
                        &mut stack,
                        binary_args.config_file,
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
                    binary_args.plugin_file,
                    binary_args.config_file,
                    binary_args.env_file,
                    binary_args.login_shell.is_some(),
                );

                let ret_val = evaluate_repl(
                    &mut engine_state,
                    &mut stack,
                    config_files::NUSHELL_FOLDER,
                    binary_args.execute,
                );
                info!("repl eval {}:{}:{}", file!(), line!(), column!());

                ret_val
            }
        }
        Err(_) => std::process::exit(1),
    }
}

fn setup_config(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    #[cfg(feature = "plugin")] plugin_file: Option<Spanned<String>>,
    config_file: Option<Spanned<String>>,
    env_file: Option<Spanned<String>>,
    is_login_shell: bool,
) {
    #[cfg(feature = "plugin")]
    read_plugin_file(engine_state, stack, plugin_file, NUSHELL_FOLDER);

    info!("read_config_file {}:{}:{}", file!(), line!(), column!());

    config_files::read_config_file(engine_state, stack, env_file, true);
    config_files::read_config_file(engine_state, stack, config_file, false);

    if is_login_shell {
        config_files::read_loginshell_file(engine_state, stack);
    }

    // Give a warning if we see `$config` for a few releases
    {
        let working_set = StateWorkingSet::new(engine_state);
        if working_set.find_variable(b"$config").is_some() {
            println!("warning: use `let-env config = ...` instead of `let config = ...`");
        }
    }
}

fn parse_commandline_args(
    commandline_args: &str,
    engine_state: &mut EngineState,
) -> Result<NushellCliArgs, ShellError> {
    let (block, delta) = {
        let mut working_set = StateWorkingSet::new(engine_state);
        working_set.add_decl(Box::new(Nu));

        let (output, err) = parse(
            &mut working_set,
            None,
            commandline_args.as_bytes(),
            false,
            &[],
        );
        if let Some(err) = err {
            report_error(&working_set, &err);

            std::process::exit(1);
        }

        working_set.hide_decl(b"nu");
        (output, working_set.render())
    };

    engine_state.merge_delta(delta)?;

    let mut stack = Stack::new();

    // We should have a successful parse now
    if let Some(pipeline) = block.pipelines.get(0) {
        if let Some(Expression {
            expr: Expr::Call(call),
            ..
        }) = pipeline.expressions.get(0)
        {
            let redirect_stdin = call.get_named_arg("stdin");
            let login_shell = call.get_named_arg("login");
            let interactive_shell = call.get_named_arg("interactive");
            let commands: Option<Expression> = call.get_flag_expr("commands");
            let testbin: Option<Expression> = call.get_flag_expr("testbin");
            #[cfg(feature = "plugin")]
            let plugin_file: Option<Expression> = call.get_flag_expr("plugin-config");
            let config_file: Option<Expression> = call.get_flag_expr("config");
            let env_file: Option<Expression> = call.get_flag_expr("env-config");
            let log_level: Option<Expression> = call.get_flag_expr("log-level");
            let log_target: Option<Expression> = call.get_flag_expr("log-target");
            let execute: Option<Expression> = call.get_flag_expr("execute");
            let threads: Option<Value> = call.get_flag(engine_state, &mut stack, "threads")?;
            let table_mode: Option<Value> =
                call.get_flag(engine_state, &mut stack, "table-mode")?;

            fn extract_contents(
                expression: Option<Expression>,
            ) -> Result<Option<Spanned<String>>, ShellError> {
                if let Some(expr) = expression {
                    let str = expr.as_string();
                    if let Some(str) = str {
                        Ok(Some(Spanned {
                            item: str,
                            span: expr.span,
                        }))
                    } else {
                        Err(ShellError::TypeMismatch("string".into(), expr.span))
                    }
                } else {
                    Ok(None)
                }
            }

            let commands = extract_contents(commands)?;
            let testbin = extract_contents(testbin)?;
            #[cfg(feature = "plugin")]
            let plugin_file = extract_contents(plugin_file)?;
            let config_file = extract_contents(config_file)?;
            let env_file = extract_contents(env_file)?;
            let log_level = extract_contents(log_level)?;
            let log_target = extract_contents(log_target)?;
            let execute = extract_contents(execute)?;

            let help = call.has_flag("help");

            if help {
                let full_help =
                    get_full_help(&Nu.signature(), &Nu.examples(), engine_state, &mut stack);

                let _ = std::panic::catch_unwind(move || stdout_write_all_and_flush(full_help));

                std::process::exit(1);
            }

            if call.has_flag("version") {
                let version = env!("CARGO_PKG_VERSION").to_string();
                let _ = std::panic::catch_unwind(move || {
                    stdout_write_all_and_flush(format!("{}\n", version))
                });

                std::process::exit(0);
            }

            return Ok(NushellCliArgs {
                redirect_stdin,
                login_shell,
                interactive_shell,
                commands,
                testbin,
                #[cfg(feature = "plugin")]
                plugin_file,
                config_file,
                env_file,
                log_level,
                log_target,
                execute,
                threads,
                table_mode,
            });
        }
    }

    // Just give the help and exit if the above fails
    let full_help = get_full_help(&Nu.signature(), &Nu.examples(), engine_state, &mut stack);
    print!("{}", full_help);
    std::process::exit(1);
}

struct NushellCliArgs {
    redirect_stdin: Option<Spanned<String>>,
    login_shell: Option<Spanned<String>>,
    interactive_shell: Option<Spanned<String>>,
    commands: Option<Spanned<String>>,
    testbin: Option<Spanned<String>>,
    #[cfg(feature = "plugin")]
    plugin_file: Option<Spanned<String>>,
    config_file: Option<Spanned<String>>,
    env_file: Option<Spanned<String>>,
    log_level: Option<Spanned<String>>,
    log_target: Option<Spanned<String>>,
    execute: Option<Spanned<String>>,
    threads: Option<Value>,
    table_mode: Option<Value>,
}

#[derive(Clone)]
struct Nu;

impl Command for Nu {
    fn name(&self) -> &str {
        "nu"
    }

    fn signature(&self) -> Signature {
        let signature = Signature::build("nu")
            .usage("The nushell language and shell.")
            .switch("stdin", "redirect the stdin", None)
            .switch("login", "start as a login shell", Some('l'))
            .switch("interactive", "start as an interactive shell", Some('i'))
            .switch("version", "print the version", Some('v'))
            .named(
                "testbin",
                SyntaxShape::String,
                "run internal test binary",
                None,
            )
            .named(
                "commands",
                SyntaxShape::String,
                "run the given commands and then exit",
                Some('c'),
            )
            .named(
                "config",
                SyntaxShape::String,
                "start with an alternate config file",
                None,
            )
            .named(
                "env-config",
                SyntaxShape::String,
                "start with an alternate environment config file",
                None,
            )
            .named(
                "log-level",
                SyntaxShape::String,
                "log level for diagnostic logs (error, warn, info, debug, trace). Off by default",
                None,
            )
            .named(
                "log-target",
                SyntaxShape::String,
                "set the target for the log to output. stdout, stderr(default), mixed or file",
                None,
            )
            .named(
                "execute",
                SyntaxShape::String,
                "run the given commands and then enter an interactive shell",
                Some('e'),
            )
            .named(
                "threads",
                SyntaxShape::Int,
                "threads to use for parallel commands",
                Some('t'),
            )
            .named(
                "table-mode",
                SyntaxShape::String,
                "the table mode to use. rounded is default.",
                Some('m'),
            )
            .optional(
                "script file",
                SyntaxShape::Filepath,
                "name of the optional script file to run",
            )
            .rest(
                "script args",
                SyntaxShape::String,
                "parameters to the script file",
            )
            .category(Category::System);

        #[cfg(feature = "plugin")]
        {
            signature.named(
                "plugin-config",
                SyntaxShape::String,
                "start with an alternate plugin signature file",
                None,
            )
        }

        #[cfg(not(feature = "plugin"))]
        {
            signature
        }
    }

    fn usage(&self) -> &str {
        "The nushell language and shell."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        Ok(Value::String {
            val: get_full_help(&Nu.signature(), &Nu.examples(), engine_state, stack),
            span: call.head,
        }
        .into_pipeline_data())
    }

    fn examples(&self) -> Vec<nu_protocol::Example> {
        vec![
            Example {
                description: "Run a script",
                example: "nu myfile.nu",
                result: None,
            },
            Example {
                description: "Run nushell interactively (as a shell or REPL)",
                example: "nu",
                result: None,
            },
        ]
    }
}

fn set_config_path(
    engine_state: &mut EngineState,
    cwd: &Path,
    default_config_name: &str,
    key: &str,
    config_file: &Option<Spanned<String>>,
) {
    let config_path = match config_file {
        Some(s) => canonicalize_with(&s.item, cwd).ok(),
        None => nu_path::config_dir().map(|mut p| {
            p.push(config_files::NUSHELL_FOLDER);
            p.push(default_config_name);
            p
        }),
    };

    if let Some(path) = config_path {
        engine_state.set_config_path(key, path);
    }
}
