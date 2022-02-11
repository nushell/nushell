mod commands;
mod config_files;
mod eval_file;
mod logger;
mod prompt_update;
mod reedline_config;
mod repl;
mod test_bins;
#[cfg(test)]
mod tests;
mod utils;

use crate::logger::{configure, logger};
use log::info;
use miette::Result;
use nu_command::{create_default_context, BufferedReader};
use nu_engine::{get_full_help, CallExt};
use nu_parser::parse;
use nu_protocol::{
    ast::{Call, Expr, Expression, Pipeline, Statement},
    engine::{Command, EngineState, Stack, StateWorkingSet},
    Category, Example, IntoPipelineData, PipelineData, RawStream, ShellError, Signature, Span,
    Spanned, SyntaxShape, Value, CONFIG_VARIABLE_ID,
};
use std::cell::RefCell;
use std::{
    io::{BufReader, Write},
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use utils::report_error;

thread_local! { static IS_PERF: RefCell<bool> = RefCell::new(false) }

fn main() -> Result<()> {
    // miette::set_panic_hook();
    let miette_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |x| {
        crossterm::terminal::disable_raw_mode().expect("unable to disable raw mode");
        miette_hook(x);
    }));

    // Get initial current working directory.
    let init_cwd = utils::get_init_cwd();
    let mut engine_state = create_default_context(&init_cwd);

    // Custom additions
    let delta = {
        let mut working_set = nu_protocol::engine::StateWorkingSet::new(&engine_state);
        working_set.add_decl(Box::new(nu_cli::NuHighlight));

        working_set.render()
    };
    let _ = engine_state.merge_delta(delta, None, &init_cwd);

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

    let mut args_to_nushell = vec![];
    let mut script_name = String::new();
    let mut args_to_script = vec![];

    // Would be nice if we had a way to parse this. The first flags we see will be going to nushell
    // then it'll be the script name
    // then the args to the script

    let mut collect_arg_nushell = false;
    for arg in std::env::args().skip(1) {
        if !script_name.is_empty() {
            args_to_script.push(if arg.contains(' ') {
                format!("'{}'", arg)
            } else {
                arg
            });
        } else if collect_arg_nushell {
            args_to_nushell.push(if arg.contains(' ') {
                format!("'{}'", arg)
            } else {
                arg
            });
            collect_arg_nushell = false;
        } else if arg.starts_with('-') {
            // Cool, it's a flag
            if arg == "-c"
                || arg == "--commands"
                || arg == "--testbin"
                || arg == "--develop"
                || arg == "--debug"
                || arg == "--loglevel"
                || arg == "--config-file"
                || arg == "--perf"
                || arg == "--threads"
            {
                collect_arg_nushell = true;
            }
            args_to_nushell.push(arg);
        } else {
            // Our script file
            script_name = arg;
        }
    }

    args_to_nushell.insert(0, "nu".into());

    let nushell_commandline_args = args_to_nushell.join(" ");

    let parsed_nu_cli_args =
        parse_commandline_args(&nushell_commandline_args, &init_cwd, &mut engine_state);

    match parsed_nu_cli_args {
        Ok(binary_args) => {
            if let Some(t) = binary_args.threads {
                // 0 means to let rayon decide how many threads to use
                let threads = t.as_i64().unwrap_or(0);
                rayon::ThreadPoolBuilder::new()
                    .num_threads(threads as usize)
                    .build_global()
                    .expect("error setting number of threads");
            }

            set_is_perf_value(binary_args.perf);

            if binary_args.perf {
                // if we started in perf mode show only the info logs
                // TODO: what happens when the config log_level is read?
                logger(|builder| {
                    configure("info", builder)?;
                    Ok(())
                })?;
                info!("start logging {}:{}:{}", file!(), line!(), column!());
            }

            if let Some(testbin) = &binary_args.testbin {
                // Call out to the correct testbin
                match testbin.item.as_str() {
                    "echo_env" => test_bins::echo_env(),
                    "cococo" => test_bins::cococo(),
                    "meow" => test_bins::meow(),
                    "iecho" => test_bins::iecho(),
                    "fail" => test_bins::fail(),
                    "nonu" => test_bins::nonu(),
                    "chop" => test_bins::chop(),
                    "repeater" => test_bins::repeater(),
                    _ => std::process::exit(1),
                }
                std::process::exit(0)
            }
            let input = if let Some(redirect_stdin) = &binary_args.redirect_stdin {
                let stdin = std::io::stdin();
                let buf_reader = BufReader::new(stdin);

                PipelineData::RawStream(
                    RawStream::new(
                        Box::new(BufferedReader::new(buf_reader)),
                        Some(ctrlc),
                        redirect_stdin.span,
                    ),
                    redirect_stdin.span,
                    None,
                )
            } else {
                PipelineData::new(Span::new(0, 0))
            };

            if is_perf_true() {
                info!("redirect_stdin {}:{}:{}", file!(), line!(), column!());
            }

            if let Some(commands) = &binary_args.commands {
                let ret_val = commands::evaluate(commands, &init_cwd, &mut engine_state, input);
                if is_perf_true() {
                    info!("-c command execution {}:{}:{}", file!(), line!(), column!());
                }

                ret_val
            } else if !script_name.is_empty() && binary_args.interactive_shell.is_none() {
                let ret_val = eval_file::evaluate(
                    script_name,
                    &args_to_script,
                    init_cwd,
                    &mut engine_state,
                    input,
                );
                if is_perf_true() {
                    info!("eval_file execution {}:{}:{}", file!(), line!(), column!());
                }

                ret_val
            } else {
                let ret_val = repl::evaluate(&mut engine_state);
                if is_perf_true() {
                    info!("repl eval {}:{}:{}", file!(), line!(), column!());
                }

                ret_val
            }
        }
        Err(_) => std::process::exit(1),
    }
}

fn parse_commandline_args(
    commandline_args: &str,
    init_cwd: &Path,
    engine_state: &mut EngineState,
) -> Result<NushellCliArgs, ShellError> {
    let (block, delta) = {
        let mut working_set = StateWorkingSet::new(engine_state);
        working_set.add_decl(Box::new(Nu));

        let (output, err) = parse(&mut working_set, None, commandline_args.as_bytes(), false);
        if let Some(err) = err {
            report_error(&working_set, &err);

            std::process::exit(1);
        }

        working_set.hide_decl(b"nu");
        (output, working_set.render())
    };

    let _ = engine_state.merge_delta(delta, None, init_cwd);

    let mut stack = Stack::new();
    stack.add_var(
        CONFIG_VARIABLE_ID,
        Value::Record {
            cols: vec![],
            vals: vec![],
            span: Span::new(0, 0),
        },
    );

    // We should have a successful parse now
    if let Some(Statement::Pipeline(Pipeline { expressions })) = block.stmts.get(0) {
        if let Some(Expression {
            expr: Expr::Call(call),
            ..
        }) = expressions.get(0)
        {
            let redirect_stdin = call.get_named_arg("stdin");
            let login_shell = call.get_named_arg("login");
            let interactive_shell = call.get_named_arg("interactive");
            let commands: Option<Expression> = call.get_flag_expr("commands");
            let testbin: Option<Expression> = call.get_flag_expr("testbin");
            let perf = call.has_flag("perf");
            let threads: Option<Value> = call.get_flag(engine_state, &mut stack, "threads")?;

            let commands = if let Some(expression) = commands {
                let contents = engine_state.get_span_contents(&expression.span);

                Some(Spanned {
                    item: String::from_utf8_lossy(contents).to_string(),
                    span: expression.span,
                })
            } else {
                None
            };

            let testbin = if let Some(expression) = testbin {
                let contents = engine_state.get_span_contents(&expression.span);

                Some(Spanned {
                    item: String::from_utf8_lossy(contents).to_string(),
                    span: expression.span,
                })
            } else {
                None
            };

            let help = call.has_flag("help");

            if help {
                let full_help =
                    get_full_help(&Nu.signature(), &Nu.examples(), engine_state, &mut stack);

                let _ = std::panic::catch_unwind(move || {
                    let stdout = std::io::stdout();
                    let mut stdout = stdout.lock();
                    let _ = stdout.write_all(full_help.as_bytes());
                });

                std::process::exit(1);
            }

            return Ok(NushellCliArgs {
                redirect_stdin,
                login_shell,
                interactive_shell,
                commands,
                testbin,
                perf,
                threads,
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
    #[allow(dead_code)]
    login_shell: Option<Spanned<String>>,
    interactive_shell: Option<Spanned<String>>,
    commands: Option<Spanned<String>>,
    testbin: Option<Spanned<String>>,
    perf: bool,
    threads: Option<Value>,
}

#[derive(Clone)]
struct Nu;

impl Command for Nu {
    fn name(&self) -> &str {
        "nu"
    }

    fn signature(&self) -> Signature {
        Signature::build("nu")
            .desc("The nushell language and shell.")
            .switch("stdin", "redirect the stdin", None)
            .switch("login", "start as a login shell", Some('l'))
            .switch("interactive", "start as an interactive shell", Some('i'))
            .switch(
                "perf",
                "start and print performance metrics during startup",
                Some('p'),
            )
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
            .optional(
                "script file",
                SyntaxShape::Filepath,
                "name of the optional script file to run",
            )
            .named(
                "threads",
                SyntaxShape::Int,
                "threads to use for parallel commands",
                Some('t'),
            )
            .rest(
                "script args",
                SyntaxShape::String,
                "parameters to the script file",
            )
            .category(Category::System)
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

pub fn is_perf_true() -> bool {
    IS_PERF.with(|value| *value.borrow())
}

// #[allow(dead_code)]
// fn is_perf_value() -> bool {
//     IS_PERF.with(|value| *value.borrow())
// }

fn set_is_perf_value(value: bool) {
    IS_PERF.with(|new_value| {
        *new_value.borrow_mut() = value;
    });
}
